// SPDX-License-Identifier: Apache-2.0
use eve_core::EveError;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::Duration;

/// JSON-RPC 2.0 request envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: u64,
}

/// JSON-RPC 2.0 response envelope.
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

/// Newline-delimited JSON-RPC client over any async read/write stream.
pub struct JsonRpcClient {
    reader: BufReader<Box<dyn tokio::io::AsyncRead + Unpin + Send>>,
    writer: Box<dyn tokio::io::AsyncWrite + Unpin + Send>,
    next_id: u64,
}

impl JsonRpcClient {
    pub fn new(
        reader: impl tokio::io::AsyncRead + Unpin + Send + 'static,
        writer: impl tokio::io::AsyncWrite + Unpin + Send + 'static,
    ) -> Self {
        JsonRpcClient {
            reader: BufReader::new(Box::new(reader)),
            writer: Box::new(writer),
            next_id: 1,
        }
    }

    /// Connect via Windows named pipe.
    #[cfg(windows)]
    pub async fn connect_pipe(pipe_name: &str) -> Result<Self, EveError> {
        use tokio::net::windows::named_pipe::ClientOptions;
        let pipe = ClientOptions::new().open(pipe_name).map_err(|e| {
            EveError::Transport(format!("Failed to open pipe {}: {}", pipe_name, e))
        })?;
        let (r, w) = tokio::io::split(pipe);
        Ok(Self::new(r, w))
    }

    /// Connect via Unix domain socket.
    #[cfg(not(windows))]
    pub async fn connect_socket(socket_path: &str) -> Result<Self, EveError> {
        let stream = tokio::net::UnixStream::connect(socket_path)
            .await
            .map_err(|e| {
                EveError::Transport(format!("Failed to connect to {}: {}", socket_path, e))
            })?;
        let (r, w) = tokio::io::split(stream);
        Ok(Self::new(r, w))
    }

    /// Send a JSON-RPC request and return the decoded result.
    pub async fn request<T: DeserializeOwned>(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<T, EveError> {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: self.next_id,
        };
        self.next_id += 1;

        let mut req_bytes =
            serde_json::to_vec(&req).map_err(|e| EveError::ExtractionParseFailed(e.to_string()))?;
        req_bytes.push(b'\n');

        self.writer
            .write_all(&req_bytes)
            .await
            .map_err(|e| EveError::Transport(e.to_string()))?;

        let mut line = String::new();
        tokio::time::timeout(Duration::from_secs(30), self.reader.read_line(&mut line))
            .await
            .map_err(|_| EveError::Transport("tpt-spark did not respond within 30s".into()))?
            .map_err(|e| EveError::Transport(e.to_string()))?;

        let response: JsonRpcResponse = serde_json::from_str(line.trim())
            .map_err(|e| EveError::ExtractionParseFailed(e.to_string()))?;

        if let Some(err) = response.error {
            return Err(EveError::Rpc {
                code: err.code,
                message: err.message,
            });
        }

        response
            .result
            .ok_or_else(|| EveError::ExtractionParseFailed("No result in response".to_string()))
            .and_then(|v| {
                serde_json::from_value(v)
                    .map_err(|e| EveError::ExtractionParseFailed(e.to_string()))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn round_trips_request_response() {
        let request = json!({"result": 42, "id": 1, "jsonrpc": "2.0", "error": null});
        let response_bytes = format!("{}\n", request);

        let (server_reader, client_writer) = tokio::io::duplex(1024);
        let (client_reader, mut server_writer) = tokio::io::duplex(1024);

        tokio::spawn(async move {
            let mut buf = String::new();
            let mut reader = BufReader::new(server_reader);
            reader.read_line(&mut buf).await.unwrap();
            server_writer
                .write_all(response_bytes.as_bytes())
                .await
                .unwrap();
        });

        let mut client = JsonRpcClient::new(client_reader, client_writer);
        let result: u64 = client.request("test_method", json!({})).await.unwrap();
        assert_eq!(result, 42);
    }
}
