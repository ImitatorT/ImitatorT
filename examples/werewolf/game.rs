//! 狼人杀游戏逻辑
//!
//! 完全独立于框架，只使用框架提供的 API

use anyhow::{Context, Result};
use imitatort_stateless_company::{Agent, Message, VirtualCompany};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

use crate::roles::{Faction, Role};

/// 游戏状态
#[derive(Debug, Clone)]
pub struct GameState {
    /// 当前天数
    pub day: u32,
    /// 当前阶段
    pub phase: GamePhase,
    /// 存活玩家
    pub alive_players: Vec<String>,
    /// 死亡玩家
    pub dead_players: Vec<String>,
    /// 投票结果
    pub votes: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum GamePhase {
    Day,
    Night,
}

impl GameState {
    pub fn new(players: Vec<String>) -> Self {
        Self {
            day: 1,
            phase: GamePhase::Day,
            alive_players: players,
            dead_players: vec![],
            votes: HashMap::new(),
        }
    }

    pub fn is_alive(&self, player: &str) -> bool {
        self.alive_players.contains(&player.to_string())
    }

    pub fn kill_player(&mut self, player: &str) {
        self.alive_players.retain(|p| p != player);
        self.dead_players.push(player.to_string());
    }
}

/// 狼人杀游戏控制器
pub struct WerewolfGame {
    company: VirtualCompany,
    state: GameState,
}

impl WerewolfGame {
    /// 创建新的游戏
    pub fn new(company: VirtualCompany) -> Self {
        // 从公司获取所有玩家（除了主持人）
        let players: Vec<String> = company
            .list_agents()
            .into_iter()
            .filter(|a| a.id() != "host-001")
            .map(|a| a.id().to_string())
            .collect();

        let state = GameState::new(players);

        Self { company, state }
    }

    /// 初始化游戏
    pub async fn initialize(&mut self) -> Result<()> {
        info!("=== 初始化狼人杀游戏 ===");

        // 创建公共群聊（全员）
        let all_members: Vec<String> = self.state.alive_players.clone();
        self.company
            .create_group("public", "公共会议室", "host-001", all_members)
            .await?;
        info!("创建公共群聊: 公共会议室");

        // 创建狼人密谋室（只有狼人）
        let werewolves: Vec<String> = self
            .state
            .alive_players
            .iter()
            .filter(|id| id.contains("werewolf"))
            .cloned()
            .collect();

        if !werewolves.is_empty() {
            self.company
                .create_group("werewolf", "狼人密谋室", "werewolf-001", werewolves)
                .await?;
            info!("创建狼人密谋室");
        }

        info!(
            "游戏初始化完成，共 {} 名玩家",
            self.state.alive_players.len()
        );
        Ok(())
    }

    /// 运行游戏主循环
    pub async fn run(&mut self) -> Result<()> {
        info!("=== 狼人杀游戏开始 ===");

        // 游戏开始公告
        self.announce("游戏开始！请大家遵守规则，享受游戏。")
            .await?;

        // 游戏循环
        loop {
            match self.state.phase {
                GamePhase::Day => self.run_day_phase().await?,
                GamePhase::Night => self.run_night_phase().await?,
            }

            // 检查游戏结束
            if self.check_game_end() {
                break;
            }

            // 切换阶段
            self.state.phase = match self.state.phase {
                GamePhase::Day => {
                    self.state.day += 1;
                    GamePhase::Night
                }
                GamePhase::Night => GamePhase::Day,
            };
        }

        // 游戏结束
        self.end_game().await?;

        Ok(())
    }

    /// 白天阶段
    async fn run_day_phase(&mut self) -> Result<()> {
        info!("=== 第 {} 天 - 白天 ===", self.state.day);

        // 宣布白天开始
        self.announce(&format!(
            "第 {} 天白天开始。存活玩家：{:?}",
            self.state.day, self.state.alive_players
        ))
        .await?;

        // 讨论阶段
        self.discussion_phase().await?;

        // 投票阶段
        self.voting_phase().await?;

        Ok(())
    }

    /// 夜晚阶段
    async fn run_night_phase(&mut self) -> Result<()> {
        info!("=== 第 {} 天 - 夜晚 ===", self.state.day);

        // 宣布夜晚开始
        self.announce("夜晚开始，各角色请行动。").await?;

        // 狼人行动
        self.werewolf_phase().await?;

        // 预言家行动
        self.seer_phase().await?;

        // 女巫行动
        self.witch_phase().await?;

        Ok(())
    }

    /// 讨论阶段
    async fn discussion_phase(&mut self) -> Result<()> {
        info!("进入讨论阶段");

        // 每个存活玩家发言
        for player_id in self.state.alive_players.clone() {
            if let Some(agent) = self.company.get_agent(&player_id) {
                let prompt = format!(
                    "现在是第 {} 天白天讨论阶段，请发表你的看法。",
                    self.state.day
                );

                match agent.run(&prompt).await {
                    Ok(speech) => {
                        info!("[{}] {}", agent.name(), speech);

                        // 广播到公共群聊
                        let msg = imitatort_stateless_company::Message::group(&player_id, "public", &speech);
                        if let Err(e) = self.company.message_bus().send(msg).await {
                            warn!("Failed to broadcast speech: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("Agent {} failed to speak: {}", player_id, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// 投票阶段
    async fn voting_phase(&mut self) -> Result<()> {
        info!("进入投票阶段");

        self.state.votes.clear();

        // 收集投票
        for voter_id in self.state.alive_players.clone() {
            if let Some(agent) = self.company.get_agent(&voter_id) {
                let prompt = format!(
                    "现在是投票阶段，存活玩家有：{:?}。请选择你要投票处决的玩家，直接回答玩家ID。",
                    self.state.alive_players
                );

                match agent.run(&prompt).await {
                    Ok(vote_result) => {
                        let voted_for = self.parse_vote(&vote_result);

                        if let Some(target) = voted_for {
                            if self.state.is_alive(&target) && target != voter_id {
                                self.state.votes.insert(voter_id.clone(), target.clone());
                                info!("[{}] 投票给 [{}]", voter_id, target);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Agent {} failed to vote: {}", voter_id, e);
                    }
                }
            }
        }

        // 处理投票结果
        self.resolve_voting().await?;

        Ok(())
    }

    /// 解析投票结果
    fn parse_vote(&self, response: &str) -> Option<String> {
        // 查找响应中包含的玩家ID
        for player_id in &self.state.alive_players {
            if response.contains(player_id) {
                return Some(player_id.clone());
            }
        }
        None
    }

    /// 处理投票结果
    async fn resolve_voting(&mut self) -> Result<()> {
        // 统计票数
        let mut vote_count: HashMap<String, u32> = HashMap::new();
        for target in self.state.votes.values() {
            *vote_count.entry(target.clone()).or_insert(0) += 1;
        }

        // 找出得票最多的玩家
        let mut max_votes = 0;
        let mut executed = None;

        for (player, count) in vote_count {
            if count > max_votes {
                max_votes = count;
                executed = Some(player);
            }
        }

        if let Some(player) = executed {
            self.state.kill_player(&player);
            self.announce(&format!("{} 被投票处决（{} 票）", player, max_votes))
                .await?;
        } else {
            self.announce("投票结果：无人被处决").await?;
        }

        Ok(())
    }

    /// 狼人阶段
    async fn werewolf_phase(&mut self) -> Result<()> {
        info!("狼人行动阶段");

        let werewolves: Vec<String> = self
            .state
            .alive_players
            .iter()
            .filter(|id| id.contains("werewolf"))
            .cloned()
            .collect();

        if werewolves.is_empty() {
            info!("没有存活的狼人");
            return Ok(());
        }

        // 狼人在密谋室讨论
        if let Some(agent) = self.company.get_agent(&werewolves[0]) {
            let prompt = format!("现在是夜晚，你是狼人。请选择你要杀害的目标，直接回答玩家ID。");

            match agent.run(&prompt).await {
                Ok(target_result) => {
                    if let Some(target) = self.parse_vote(&target_result) {
                        if self.state.is_alive(&target) {
                            self.state.kill_player(&target);
                            self.announce(&format!("天亮了，{} 昨晚被杀害了", target))
                                .await?;
                        }
                    }
                }
                Err(e) => {
                    warn!("Werewolf failed to act: {}", e);
                }
            }
        }

        Ok(())
    }

    /// 预言家阶段
    async fn seer_phase(&mut self) -> Result<()> {
        if !self.state.is_alive("seer-001") {
            return Ok(());
        }

        info!("预言家行动阶段");

        if let Some(agent) = self.company.get_agent("seer-001") {
            let prompt = "现在是夜晚，你是预言家。请选择你要查验身份的玩家，直接回答玩家ID。";

            match agent.run(prompt).await {
                Ok(check_result) => {
                    info!("[预言家] 查验了: {}", check_result);
                    // 预言家自己知道结果，不公开
                }
                Err(e) => {
                    warn!("Seer failed to act: {}", e);
                }
            }
        }

        Ok(())
    }

    /// 女巫阶段
    async fn witch_phase(&mut self) -> Result<()> {
        if !self.state.is_alive("witch-001") {
            return Ok(());
        }

        info!("女巫行动阶段");
        // 简化：女巫逻辑略过

        Ok(())
    }

    /// 检查游戏结束
    fn check_game_end(&self) -> bool {
        let werewolf_count = self
            .state
            .alive_players
            .iter()
            .filter(|p| p.contains("werewolf"))
            .count();

        let good_count = self.state.alive_players.len() - werewolf_count;

        werewolf_count == 0 || good_count == 0
    }

    /// 结束游戏
    async fn end_game(&mut self) -> Result<()> {
        let werewolf_count = self
            .state
            .alive_players
            .iter()
            .filter(|p| p.contains("werewolf"))
            .count();

        let winner = if werewolf_count == 0 {
            "好人阵营"
        } else {
            "狼人阵营"
        };

        self.announce(&format!(
            "游戏结束！{} 获胜！存活玩家：{:?}，死亡玩家：{:?}",
            winner, self.state.alive_players, self.state.dead_players
        ))
        .await?;

        Ok(())
    }

    /// 发送公告（通过主持人）
    async fn announce(&self, message: &str) -> Result<()> {
        info!("[公告] {}", message);

        // 通过广播发送
        self.company.broadcast("host-001", message)?;

        Ok(())
    }
}
