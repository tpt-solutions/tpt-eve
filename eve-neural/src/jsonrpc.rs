use eve_core::EveError;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[cfg(windows)]
use tokio::net::TcpStream;

#[cfg(unix)]
use tokio::net::UnixStream;

/// JSON-RPC request/response for communication with tpt-spark.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<JsonRpcError>,
    pub id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

/// JSON-RPC client for communicating with tpt-spark (Unix socket) or other backends.
pub struct JsonRpcClient {
    #[cfg(windows)]
    stream: TcpStream,
    #[cfg(unix)]
    stream: UnixStream,
    next_id: u64,
}

impl JsonRpcClient {
    #[cfg(windows)]
    pub async fn connect(_pipe_path: &str) -> Result<Self, EveError> {
        // On Windows, use TCP for now (tpt-spark may use TCP or named pipe)
        let stream = TcpStream::connect("127.0.0.1:0")
            .await
            .map_err(|e| EveError::Transport(format!("Failed to connect: {}", e)))?;
        Ok(JsonRpcClient {
            stream,
            next_id: 1,
        })
    }

    #[cfg(unix)]
    pub async fn connect(socket_path: &str) -> Result<Self, EveError> {
        let stream = UnixStream::connect(socket_path)
            .await
            .map_err(|e| EveError::Transport(format!("Failed to connect to socket: {}", e)))?;
        Ok(JsonRpcClient {
            stream,
            next_id: 1,
        })
    }

    pub async fn request<T: DeserializeOwned>(&mut self, method: &str, params: serde_json::Value) -> Result<T, EveError> {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: self.next_id,
        };
        self.next_id += 1;

        let req_bytes = serde_json::to_vec(&req).map_err(|e| EveError::ExtractionParseFailed(e.to_string()))?;
        
        self.stream.write_all(&req_bytes).await.map_err(|e| EveError::Transport(e.to_string()))?;
        self.stream.write_all(b"\n").await.map_err(|e| EveError::Transport(e.to_string()))?;

        let mut response_bytes = Vec::new();
        self.stream.read_to_end(&mut response_bytes).await.map_err(|e| EveError::Transport(e.to_string()))?;

        let response: JsonRpcResponse = serde_json::from_slice(&response_bytes)
            .map_err(|e| EveError::ExtractionParseFailed(e.to_string()))?;
        
        if let Some(err) = response.error {
            return Err(EveError::Rpc { code: err.code, message: err.message });
        }

        response.result
            .ok_or_else(|| EveError::ExtractionParseFailed("No result in response".to_string()))
            .and_then(|v| serde_json::from_value(v).map_err(|e| EveError::ExtractionParseFailed(e.to_string())))
    }
}