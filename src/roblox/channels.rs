use serde::{Deserialize, Serialize};

use super::{Client, Deployment, Error, Platform};

const KNOWN_CHANNELS: [&str; 3] = ["LIVE", "zCanary", "zIntegration"];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Channel {
    pub name: String,
    pub public: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deployment: Option<Deployment>,
}

pub fn known_channels() -> Vec<String> {
    KNOWN_CHANNELS.into_iter().map(str::to_owned).collect()
}

impl Client {
    pub async fn get_public_channels(&self, platform: Platform) -> Result<Vec<Channel>, Error> {
        let mut channels = Vec::with_capacity(KNOWN_CHANNELS.len());

        for channel_name in KNOWN_CHANNELS {
            let deployment = match self.get_deployment(platform, channel_name).await {
                Ok(deployment) => deployment,
                Err(err) => {
                    if err
                        .status_code()
                        .is_some_and(super::client::is_invalid_channel_status)
                    {
                        continue;
                    }

                    return Err(Error::ChannelCheck {
                        channel: channel_name.to_owned(),
                        source: Box::new(err),
                    });
                }
            };

            channels.push(Channel {
                name: channel_name.to_owned(),
                public: true,
                deployment: Some(deployment),
            });
        }

        Ok(channels)
    }
}

#[cfg(test)]
mod tests {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    use super::{Client, Platform};

    async fn mock_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock server");
        let address = listener.local_addr().expect("mock server address");

        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };

                tokio::spawn(async move {
                    let mut request = Vec::new();
                    let mut buffer = [0_u8; 1024];

                    loop {
                        let Ok(read) = socket.read(&mut buffer).await else {
                            return;
                        };

                        if read == 0 {
                            return;
                        }

                        request.extend_from_slice(&buffer[..read]);

                        if request.windows(4).any(|window| window == b"\r\n\r\n") {
                            break;
                        }
                    }

                    let request = String::from_utf8_lossy(&request);

                    let (status, body) = if request.contains("/channel/zCanary")
                        || request.contains("/channel/zIntegration")
                    {
                        ("401 Unauthorized", "")
                    } else {
                        (
                            "200 OK",
                            r#"{"version":"0.1.0","clientVersionUpload":"version-test"}"#,
                        )
                    };

                    let response = format!(
                        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    );

                    let _ = socket.write_all(response.as_bytes()).await;
                });
            }
        });

        format!("http://{address}")
    }

    #[tokio::test]
    async fn skips_channels_that_return_invalid_statuses() {
        let client = Client::with_base_url(mock_server().await);

        let channels = client
            .get_public_channels(Platform::WindowsPlayer)
            .await
            .expect("public channels should succeed");

        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].name, "LIVE");
        assert_eq!(
            channels[0]
                .deployment
                .as_ref()
                .map(|deployment| deployment.version.as_str()),
            Some("0.1.0"),
        );
    }
}
