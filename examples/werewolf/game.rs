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
        info!("🎮 ╔══════════════════════════════════════════════════════════╗");
        info!("🎮 ║              🐺 狼人杀游戏 - 初始化                     ║");
        info!("🎮 ╚══════════════════════════════════════════════════════════╝");

        // 创建公共群聊（全员）
        let all_members: Vec<String> = self.state.alive_players.clone();
        self.company
            .create_group("public", "公共会议室", "host-001", all_members)
            .await?;
        info!("📢 创建公共群聊: 公共会议室");

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
            info!("🐺 创建狼人密谋室（成员: {:?}）", werewolves);
        }

        info!("🎮 ╔══════════════════════════════════════════════════════════╗");
        info!("🎮 ║  游戏配置:                                              ║");
        info!("🎮 ║    总玩家: {} 人                                        ║", self.state.alive_players.len());
        info!("🎮 ║    狼人: {} 人                                          ║", werewolves.len());
        info!("🎮 ║    神职: {} 人                                          ║", self.state.alive_players.len() - werewolves.len() - 4); // 4个村民
        info!("🎮 ║    村民: 4 人                                           ║");
        info!("🎮 ╚══════════════════════════════════════════════════════════╝");

        // 显示所有玩家
        info!("👥 玩家列表:");
        for (i, player) in self.state.alive_players.iter().enumerate() {
            let role = if player.contains("werewolf") {
                "🐺 狼人"
            } else if player.contains("seer") {
                "🔮 预言家"
            } else if player.contains("witch") {
                "🧪 女巫"
            } else if player.contains("hunter") {
                "🔫 猎人"
            } else if player.contains("host") {
                "🎤 主持人"
            } else {
                "👤 村民"
            };
            info!("   {}. {} {}", i + 1, player, role);
        }

        Ok(())
    }

    /// 运行游戏主循环
    pub async fn run(&mut self) -> Result<()> {
        info!("🎮 ╔══════════════════════════════════════════════════════════╗");
        info!("🎮 ║              🐺 狼人杀游戏正式开始！                    ║");
        info!("🎮 ╚══════════════════════════════════════════════════════════╝");

        // 游戏开始公告
        self.announce("🎮 游戏开始！请大家遵守规则，享受游戏。")
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
        info!("☀️ ╔══════════════════════════════════════════════════════════╗");
        info!("☀️ ║              第 {} 天 - 白天                           ║", self.state.day);
        info!("☀️ ╠══════════════════════════════════════════════════════════╣");
        info!("☀️ ║  存活玩家: {} 人                                       ║", self.state.alive_players.len());
        info!("☀️ ╚══════════════════════════════════════════════════════════╝");

        // 宣布白天开始
        self.announce(&format!(
            "☀️ 第 {} 天白天开始。存活玩家：{:?}",
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
        info!("🌙 ╔══════════════════════════════════════════════════════════╗");
        info!("🌙 ║              第 {} 天 - 夜晚                           ║", self.state.day);
        info!("🌙 ╠══════════════════════════════════════════════════════════╣");
        info!("🌙 ║  存活玩家: {} 人                                       ║", self.state.alive_players.len());
        info!("🌙 ╚══════════════════════════════════════════════════════════╝");

        // 宣布夜晚开始
        self.announce("🌙 夜晚开始，各角色请行动。").await?;

        // 狼人行动
        self.werewolf_phase().await?;

        // 预言家行动
        self.seer_phase().await?;

        // 女巫行动
        self.witch_phase().await?;

        info!("🌙 ╔══════════════════════════════════════════════════════════╗");
        info!("🌙 ║              第 {} 天 - 夜晚结束                       ║", self.state.day);
        info!("🌙 ╚══════════════════════════════════════════════════════════╝");

        Ok(())
    }

    /// 讨论阶段
    async fn discussion_phase(&mut self) -> Result<()> {
        info!("🗣️ === 第{}天白天 - 讨论阶段 ===", self.state.day);

        // 每个存活玩家发言
        for (idx, player_id) in self.state.alive_players.clone().iter().enumerate() {
            if let Some(agent) = self.company.get_agent(player_id) {
                let prompt = format!(
                    "现在是第 {} 天白天讨论阶段。\n存活玩家: {:?}\n请发表你的看法和推理，分析谁是狼人。",
                    self.state.day,
                    self.state.alive_players
                );

                info!("🎤 [{}/{}] {} ({}) 开始发言...",
                    idx + 1,
                    self.state.alive_players.len(),
                    agent.name(),
                    player_id
                );

                match agent.run(&prompt).await {
                    Ok(speech) => {
                        // 打印发言内容（多行格式）
                        info!("🗣️ ╔══════════════════════════════════════════════════════════╗");
                        info!("🗣️ ║  {} ({}) 的发言：", agent.name(), player_id);
                        info!("🗣️ ╠══════════════════════════════════════════════════════════╣");
                        for line in speech.lines() {
                            info!("🗣️ ║  {}", line);
                        }
                        info!("🗣️ ╚══════════════════════════════════════════════════════════╝");

                        // 广播到公共群聊
                        let msg = imitatort_stateless_company::Message::group(
                            player_id, "public", &speech,
                        );
                        if let Err(e) = self.company.message_bus().send(msg).await {
                            warn!("Failed to broadcast speech: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("❌ Agent {} failed to speak: {}", player_id, e);
                    }
                }
            }
        }

        info!("🗣️ === 讨论阶段结束 ===");
        Ok(())
    }

    /// 投票阶段
    async fn voting_phase(&mut self) -> Result<()> {
        info!("🗳️ === 第{}天 - 投票阶段 ===", self.state.day);
        info!("🗳️ 存活玩家: {:?}", self.state.alive_players);

        self.state.votes.clear();

        // 收集投票
        for voter_id in self.state.alive_players.clone() {
            if let Some(agent) = self.company.get_agent(&voter_id) {
                let prompt = format!(
                    "现在是第{}天投票阶段。\n存活玩家: {:?}\n请选择你要投票处决的玩家，直接回答玩家ID（如：villager-001）。",
                    self.state.day,
                    self.state.alive_players
                );

                info!("🗳️ [{}] 正在投票...", agent.name());

                match agent.run(&prompt).await {
                    Ok(vote_result) => {
                        info!("🗳️ [{}] 投票回复: {}", agent.name(), vote_result);

                        let voted_for = self.parse_vote(&vote_result);

                        if let Some(target) = voted_for {
                            if self.state.is_alive(&target) && target != voter_id {
                                self.state.votes.insert(voter_id.clone(), target.clone());
                                info!("✅ [{}] ➜ 投票给 [{}]", agent.name(), target);
                            } else if target == voter_id {
                                info!("⚠️ [{}] ➜ 试图投给自己（无效）", agent.name());
                            } else {
                                info!("❌ [{}] ➜ 投票目标 [{}] 已死亡", agent.name(), target);
                            }
                        } else {
                            info!("⚠️ [{}] ➜ 未能解析投票目标", agent.name());
                        }
                    }
                    Err(e) => {
                        warn!("❌ Agent {} 投票失败: {}", voter_id, e);
                    }
                }
            }
        }

        // 显示投票汇总
        info!("🗳️ === 投票汇总 ===");
        for (voter, target) in &self.state.votes {
            info!("🗳️ {} ➜ {}", voter, target);
        }

        // 处理投票结果
        self.resolve_voting().await?;

        Ok(())
    }

    /// 解析投票结果 - 智能提取玩家ID
    fn parse_vote(&self, response: &str) -> Option<String> {
        // 首先尝试精确匹配（处理直接回复ID的情况）
        let trimmed = response.trim();
        for player_id in &self.state.alive_players {
            // 精确匹配
            if trimmed == *player_id {
                return Some(player_id.clone());
            }
            // 包含匹配（用于长文本中提取ID）
            if response.contains(player_id) {
                return Some(player_id.clone());
            }
        }

        // 尝试匹配常见的ID格式（如 villager-001, werewolf-001, seer-001 等）
        let id_patterns = [
            r"(villager-\d+)",
            r"(werewolf-\d+)",
            r"(seer-\d+)",
            r"(witch-\d+)",
            r"(hunter-\d+)",
            r"(host-\d+)",
        ];

        for pattern in &id_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if let Some(caps) = regex.captures(response) {
                    if let Some(matched) = caps.get(1) {
                        let id = matched.as_str();
                        // 验证这个ID是否在存活玩家中
                        if self.state.is_alive(id) {
                            return Some(id.to_string());
                        }
                    }
                }
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

        info!("📊 ╔══════════════════════════════════════════════════════════╗");
        info!("📊 ║                    投票统计结果                        ║");
        info!("📊 ╠══════════════════════════════════════════════════════════╣");

        // 显示票数统计
        let mut vote_vec: Vec<_> = vote_count.iter().collect();
        vote_vec.sort_by(|a, b| b.1.cmp(a.1)); // 按票数降序

        for (player, count) in &vote_vec {
            info!("📊 ║  {}: {} 票", player, count);
        }
        info!("📊 ╠══════════════════════════════════════════════════════════╣");

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
            info!("📊 ║  处决结果: {} ({}票)                          ║", player, max_votes);
            info!("📊 ╚══════════════════════════════════════════════════════════╝");
            self.announce(&format!("☠️ {} 被投票处决（{} 票）", player, max_votes))
                .await?;
        } else {
            info!("📊 ║  处决结果: 无人被处决（平票或无人投票）      ║");
            info!("📊 ╚══════════════════════════════════════════════════════════╝");
            self.announce("😐 投票结果：无人被处决").await?;
        }

        Ok(())
    }

    /// 狼人阶段
    async fn werewolf_phase(&mut self) -> Result<()> {
        info!("🐺 狼人行动阶段");

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

        info!("存活狼人: {:?}", werewolves);

        // 狼人在密谋室讨论
        if let Some(agent) = self.company.get_agent(&werewolves[0]) {
            let alive_players: Vec<String> = self.state.alive_players.clone();
            let prompt = format!(
                "现在是夜晚，你是狼人。存活玩家有：{:?}。\n请选择你要杀害的目标，直接回答玩家ID（如：villager-001）。",
                alive_players
            );

            info!("🐺 [{}] 正在选择杀害目标...", agent.name());

            match agent.run(&prompt).await {
                Ok(target_result) => {
                    info!("🐺 [{}] 的决策: {}", agent.name(), target_result);

                    if let Some(target) = self.parse_vote(&target_result) {
                        if self.state.is_alive(&target) {
                            self.state.kill_player(&target);
                            info!("🐺 [{}] 选择杀死: {}", agent.name(), target);
                            self.announce(&format!("☠️ 天亮了，{} 昨晚被狼人杀害了", target))
                                .await?;
                        } else {
                            warn!("🐺 目标 {} 已经死亡，无法杀害", target);
                        }
                    } else {
                        warn!("🐺 [{}] 未能选择有效的杀害目标，原始回复: {}", agent.name(), target_result);
                    }
                }
                Err(e) => {
                    warn!("🐺 狼人行动失败: {}", e);
                }
            }
        }

        Ok(())
    }

    /// 预言家阶段
    async fn seer_phase(&mut self) -> Result<()> {
        if !self.state.is_alive("seer-001") {
            info!("🔮 预言家已死亡，跳过查验阶段");
            return Ok(());
        }

        info!("🔮 预言家行动阶段");

        if let Some(agent) = self.company.get_agent("seer-001") {
            let prompt = format!(
                "现在是夜晚，你是预言家。\n存活玩家: {:?}\n请选择你要查验身份的玩家，直接回答玩家ID（如：villager-001）。",
                self.state.alive_players
            );

            info!("🔮 [预言家] 正在选择查验目标...");

            match agent.run(&prompt).await {
                Ok(check_result) => {
                    info!("🔮 [预言家] 查验回复: {}", check_result);

                    if let Some(target) = self.parse_vote(&check_result) {
                        // 判断目标是否是狼人
                        let is_werewolf = target.contains("werewolf");
                        let identity = if is_werewolf { "🐺 狼人" } else { "👤 好人" };
                        info!("🔮 [预言家] 查验 [{}] ➜ {}", target, identity);
                    } else {
                        info!("⚠️ [预言家] 未能选择有效的查验目标");
                    }
                }
                Err(e) => {
                    warn!("❌ [预言家] 查验失败: {}", e);
                }
            }
        }

        Ok(())
    }

    /// 女巫阶段
    async fn witch_phase(&mut self) -> Result<()> {
        if !self.state.is_alive("witch-001") {
            info!("🧪 女巫已死亡，跳过行动阶段");
            return Ok(());
        }

        info!("🧪 女巫行动阶段");
        info!("🧪 [女巫] 正在考虑是否使用药水...");
        // 简化：女巫逻辑略过（可以扩展实现救人/毒人）
        info!("🧪 [女巫] 本轮选择观望");

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

        let (winner, emoji) = if werewolf_count == 0 {
            ("好人阵营", "👤")
        } else {
            ("狼人阵营", "🐺")
        };

        info!("🏆 ╔══════════════════════════════════════════════════════════╗");
        info!("🏆 ║                    🎉 游戏结束 🎉                      ║");
        info!("🏆 ╠══════════════════════════════════════════════════════════╣");
        info!("🏆 ║  {} {} 获胜！                                          ║", emoji, winner);
        info!("🏆 ╠══════════════════════════════════════════════════════════╣");
        info!("🏆 ║  存活玩家: {:?}", self.state.alive_players);
        info!("🏆 ║  死亡玩家: {:?}", self.state.dead_players);
        info!("🏆 ╚══════════════════════════════════════════════════════════╝");

        self.announce(&format!(
            "🎉 游戏结束！{} {} 获胜！存活：{}人，死亡：{}人",
            emoji, winner, self.state.alive_players.len(), self.state.dead_players.len()
        ))
        .await?;

        Ok(())
    }

    /// 发送公告（通过主持人）
    async fn announce(&self, message: &str) -> Result<()> {
        info!("📢 ╔══════════════════════════════════════════════════════════╗");
        info!("📢 ║  📢 公告: {}", message);
        info!("📢 ╚══════════════════════════════════════════════════════════╝");

        // 通过广播发送
        self.company.broadcast("host-001", message)?;

        Ok(())
    }
}
