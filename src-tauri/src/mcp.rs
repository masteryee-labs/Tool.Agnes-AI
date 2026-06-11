use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::config::McpServerConfig;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

pub struct McpServerProcess {
    pub name: String,
    #[allow(dead_code)]
    child: Child,
    stdin: tokio::process::ChildStdin,
    reader: BufReader<tokio::process::ChildStdout>,
    request_id: i64,
}

impl McpServerProcess {
    pub async fn new(config: &McpServerConfig) -> Result<Self, String> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
            .envs(&config.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("無法啟動 MCP Server {}: {}", config.name, e))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "無法取得 stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "無法取得 stdout".to_string())?;
        let reader = BufReader::new(stdout);

        let mut server = Self {
            name: config.name.clone(),
            child,
            stdin,
            reader,
            request_id: 1,
        };

        server.initialize().await?;
        Ok(server)
    }

    async fn send_request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let id = self.request_id;
        self.request_id += 1;

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let req_str = serde_json::to_string(&request).map_err(|e| e.to_string())? + "\n";
        self.stdin
            .write_all(req_str.as_bytes())
            .await
            .map_err(|e| format!("寫入失敗: {}", e))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| format!("Flush 失敗: {}", e))?;

        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .await
            .map_err(|e| format!("讀取失敗: {}", e))?;

        let response: serde_json::Value =
            serde_json::from_str(&line).map_err(|e| {
                format!("解析 JSON 失敗: {}. 原始內容: {}", e, line)
            })?;

        if let Some(err) = response.get("error") {
            return Err(format!("MCP 錯誤: {:?}", err));
        }

        Ok(response.get("result").cloned().unwrap_or(serde_json::Value::Null))
    }

    async fn send_notification(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<(), String> {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });

        let req_str = serde_json::to_string(&notification).map_err(|e| e.to_string())? + "\n";
        self.stdin
            .write_all(req_str.as_bytes())
            .await
            .map_err(|e| format!("寫入失敗: {}", e))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| format!("Flush 失敗: {}", e))?;
        Ok(())
    }

    async fn initialize(&mut self) -> Result<(), String> {
        let params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "agnes-client",
                "version": "1.0.0",
            },
        });

        self.send_request("initialize", params).await?;
        self.send_notification("notifications/initialized", serde_json::json!({}))
            .await?;
        Ok(())
    }

    pub async fn list_tools(&mut self) -> Result<Vec<McpTool>, String> {
        let result = self
            .send_request("tools/list", serde_json::json!({}))
            .await?;
        let tools_val = result.get("tools").ok_or("缺少 tools 欄位")?;
        let tools: Vec<McpTool> =
            serde_json::from_value(tools_val.clone()).map_err(|e| e.to_string())?;
        Ok(tools)
    }

    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<String, String> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments,
        });

        let result = self
            .send_request("tools/call", params)
            .await?;
        let content_val = result.get("content").ok_or("缺少 content 欄位")?;
        let mut text_output = String::new();
        if let Some(arr) = content_val.as_array() {
            for item in arr {
                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                    text_output.push_str(text);
                }
            }
        }
        Ok(text_output)
    }
}

#[derive(Clone)]
pub struct McpManager {
    pub servers: Arc<Mutex<HashMap<String, McpServerProcess>>>,
}

impl Default for McpManager {
    fn default() -> Self {
        Self::new()
    }
}

impl McpManager {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start_server(&self, config: &McpServerConfig) -> Result<(), String> {
        let proc = McpServerProcess::new(config).await?;
        let mut servers = self.servers.lock().await;
        servers.insert(config.name.clone(), proc);
        Ok(())
    }

    pub async fn stop_server(&self, name: &str) -> Result<(), String> {
        let mut servers = self.servers.lock().await;
        if servers.remove(name).is_some() {
            Ok(())
        } else {
            Err(format!("找不到運作中的 MCP 伺服器: {}", name))
        }
    }

    pub async fn start_servers(&self, configs: &[McpServerConfig]) {
        for config in configs {
            println!("[MCP] 正在啟動伺服器: {}", config.name);
            match self.start_server(config).await {
                Ok(_) => println!("[MCP] 伺服器 {} 啟動成功", config.name),
                Err(e) => eprintln!("[MCP] 伺服器 {} 啟動失敗: {}", config.name, e),
            }
        }
    }

    pub async fn get_all_tools(&self) -> HashMap<String, Vec<McpTool>> {
        let mut servers = self.servers.lock().await;
        let mut all_tools = HashMap::new();
        for (name, proc) in servers.iter_mut() {
            match proc.list_tools().await {
                Ok(tools) => {
                    all_tools.insert(name.clone(), tools);
                }
                Err(e) => {
                    eprintln!("[MCP] 無法取得 {} 的工具列表: {}", name, e);
                }
            }
        }
        all_tools
    }

    pub async fn call_mcp_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<String, String> {
        let mut servers = self.servers.lock().await;
        let proc = servers
            .get_mut(server_name)
            .ok_or_else(|| format!("找不到 MCP 伺服器: {}", server_name))?;
        proc.call_tool(tool_name, arguments).await
    }
}
