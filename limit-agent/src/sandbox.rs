use crate::error::AgentError;
use bollard::container::{Config, CreateContainerOptions, RemoveContainerOptions};
use bollard::Docker;
use futures::StreamExt;
use tokio::time::{timeout, Duration};
pub struct DockerSandbox {
    docker: Docker,
}

impl DockerSandbox {
    /// Check if Docker daemon is available
    pub async fn check_docker_available() -> bool {
        if let Ok(docker) = Docker::connect_with_defaults() {
            if let Ok(_) = docker.ping().await {
                return true;
            }
        }
        false
    }

    /// Create a new DockerSandbox instance
    pub async fn new() -> Result<Self, AgentError> {
        let docker = Docker::connect_with_defaults()
            .map_err(|e| AgentError::SandboxError(format!("Failed to connect to Docker: {}", e)))?;

        // Verify connection
        docker.ping()
            .await
            .map_err(|e| AgentError::SandboxError(format!("Docker ping failed: {}", e)))?;

        Ok(DockerSandbox { docker })
    }

    /// Create a container with specified image
    pub async fn create_container(&self, image: &str) -> Result<String, AgentError> {
        // Default to limit-rust-sandbox:latest if not specified
        let image_name = if image.is_empty() {
            "limit-rust-sandbox:latest"
        } else {
            image
        };

        let cwd = std::env::current_dir()
            .map_err(|e| AgentError::SandboxError(format!("Failed to get current directory: {}", e)))?;

        let config = Config {
            image: Some(image_name.to_string()),
            tty: Some(false),
            open_stdin: Some(false),
            host_config: Some(bollard::models::HostConfig {
                binds: Some(vec![format!("{}:/workspace:ro", cwd.to_string_lossy())]),
                memory: Some(512 * 1024 * 1024), // 512MB
                network_mode: Some("none".to_string()),
                ..Default::default()
            }),
            working_dir: Some("/workspace".to_string()),
            ..Default::default()
        };

        let uuid = uuid::Uuid::new_v4().to_string();
        let short_uuid = uuid.chars().take(8).collect::<String>();
        let options = Some(CreateContainerOptions {
            name: format!("limit-sandbox-{}", short_uuid),
            ..Default::default()
        });

        let container = self
            .docker
            .create_container(options, config)
            .await
            .map_err(|e| AgentError::SandboxError(format!("Failed to create container: {}", e)))?;

        Ok(container.id)
    }

    /// Execute a command in the container

    /// Execute a command in the container
    pub async fn execute_in_container(
        &self,
        container: &str,
        cmd: &[String],
    ) -> Result<String, AgentError> {
        // Create exec instance
        let exec_config = bollard::exec::CreateExecOptions {
            cmd: Some(cmd.to_vec()),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };

        let exec = self
            .docker
            .create_exec(container, exec_config)
            .await
            .map_err(|e| AgentError::SandboxError(format!("Failed to create exec: {}", e)))?;

        // Start exec with 60s timeout
        let exec_start = bollard::exec::StartExecOptions {
            detach: false,
            ..Default::default()
        };

        let result = timeout(
            Duration::from_secs(60),
            self.docker.start_exec(&exec.id, Some(exec_start)),
        )
        .await
        .map_err(|_| AgentError::SandboxError("Command execution timed out".to_string()))?
        .map_err(|e| AgentError::SandboxError(format!("Failed to start exec: {}", e)))?;

        let output = match result {
            bollard::exec::StartExecResults::Attached { output, .. } => {
                let mut full_output = Vec::new();
                
                let mut stream = output;
                while let Some(result) = stream.next().await {
                    let chunk = result.map_err(|e| AgentError::SandboxError(format!("Failed to read output: {}", e)))?;
                    full_output.extend_from_slice(&chunk.into_bytes());
                }
                
                String::from_utf8_lossy(&full_output).to_string()
            }
            bollard::exec::StartExecResults::Detached => String::new(),
        };

        Ok(output)
    }

    /// Start a container
    pub async fn start_container(&self, container: &str) -> Result<(), AgentError> {
        self.docker
            .start_container::<String>(container, None)
            .await
            .map_err(|e| AgentError::SandboxError(format!("Failed to start container: {}", e)))?;

        Ok(())
    }

    /// Stop and remove a container
    pub async fn cleanup_container(&self, container: &str) {
        let remove_options = RemoveContainerOptions {
            force: true,
            v: true,
            ..Default::default()
        };

        let _ = self.docker.remove_container(container, Some(remove_options)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_docker_available() {
        let available = DockerSandbox::check_docker_available().await;
        // Test doesn't assert - just verifies it runs without panicking
        println!("Docker available: {}", available);
    }

    #[tokio::test]
    async fn test_docker_not_available_returns_error() {
        // This test verifies that the sandbox handles missing Docker gracefully
        if !DockerSandbox::check_docker_available().await {
            let result = DockerSandbox::new().await;
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    #[ignore] // Requires Docker to be running
    async fn test_create_and_cleanup_container() {
        if !DockerSandbox::check_docker_available().await {
            return;
        }

        let sandbox = DockerSandbox::new().await.unwrap();
        let container_id = sandbox.create_container("alpine:latest").await.unwrap();

        // Cleanup should work even if container wasn't started
        sandbox.cleanup_container(&container_id).await;
    }

    #[tokio::test]
    #[ignore] // Requires Docker and alpine image
    async fn test_execute_command() {
        if !DockerSandbox::check_docker_available().await {
            return;
        }

        let sandbox = DockerSandbox::new().await.unwrap();
        let container_id = sandbox.create_container("alpine:latest").await.unwrap();

        sandbox.start_container(&container_id).await.unwrap();

        let output = sandbox
            .execute_in_container(&container_id, &["echo".to_string(), "hello".to_string()])
            .await
            .unwrap();

        assert!(output.contains("hello"));

        sandbox.cleanup_container(&container_id).await;
    }
}
