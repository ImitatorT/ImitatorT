---
name: largomst-ducksearch
description: 使用 DuckDuckGo 进行网页搜索和内容提取的命令行工具。当用户需要搜索网络信息、查找资料、获取网页内容时使用此
  skill。触发场景包括：(1) 搜索网络内容 (2) 获取网页文本 (3) 使用 DuckDuckGo 搜索 (4) 抓取网页内容
---

## 快速使用

### 准备工作

在执行搜索前启用系统代理

```
export https_proxy=http://127.0.0.1:7890 http_proxy=http://127.0.0.1:7890 all_proxy=socks5://127.0.0.1:7890
```


### 搜索网络

```bash
pnpx ducksearch search "搜索关键词"
pnpx ducksearch search "Claude AI" -n 5      # 限制结果数量
pnpx ducksearch search "Claude AI" -o        # 自动打开第一个结果
```

### 获取网页内容

```bash
pnpx ducksearch fetch https://example.com
pnpx ducksearch fetch https://example.com --raw      # 原始 HTML
pnpx ducksearch fetch https://example.com -o out.txt # 保存到文件
pnpx ducksearch fetch https://example.com --json     # JSON 格式
```
