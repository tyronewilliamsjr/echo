use aws_config;
use aws_sdk_s3;
use aws_sdk_sqs;

pub struct AwsClients {
    pub sqs_client: aws_sdk_sqs::Client,
    pub s3_client: aws_sdk_s3::Client,
}

impl AwsClients {
    pub async fn new() -> AwsClients {
        let shared_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .load()
            .await;

        AwsClients {
            sqs_client: aws_sdk_sqs::Client::new(&shared_config),
            s3_client: aws_sdk_s3::Client::new(&shared_config),
        }
    }
}
