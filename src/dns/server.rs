use async_trait::async_trait;
use hickory_proto::op::{Message, MessageType, OpCode, ResponseCode};
use hickory_server::{
    authority::MessageResponseBuilder,
    server::{Request, RequestHandler, ResponseHandler, ResponseInfo},
};
use moka::sync::Cache;
use std::time::Duration;

use crate::config::Config;
use crate::dns::client::forward_to_upstream;
use core::net::SocketAddr;
use hickory_proto::rr::{Name, RecordType};
use tracing;

pub struct Handler {
    pub config: Config,
    cache: Cache<(Name, RecordType), Message>,
}

impl Clone for Handler {
    fn clone(&self) -> Self {
        Handler {
            config: self.config.clone(),
            cache: self.cache.clone(),
        }
    }
}

impl Handler {
    pub fn new(config: Config) -> Self {
        let cache = if config.cache.enabled {
            Cache::builder()
                .time_to_live(Duration::from_secs(config.cache.ttl as u64))
                .build()
        } else {
            Cache::new(0)
        };
        Handler { config, cache }
    }
}

#[async_trait]
impl RequestHandler for Handler {
    async fn handle_request<R: ResponseHandler>(
        &self,
        request: &Request,
        mut response_handle: R,
    ) -> ResponseInfo {
        let response = MessageResponseBuilder::from_message_request(request);
        let mut message = Message::new();
        message.set_id(request.id());
        message.set_message_type(MessageType::Response);
        message.set_op_code(OpCode::Query);
        if request.recursion_desired() {
            message.set_recursion_available(true);
            message.set_recursion_desired(true);
        }

        let upstreams: Vec<SocketAddr> = self
            .config
            .upstreams
            .iter()
            .filter_map(|s| match s.parse() {
                Ok(addr) => Some(addr),
                Err(e) => {
                    tracing::error!("Failed to parse upstream address {}: {}", s, e);
                    None
                }
            })
            .collect();

        if upstreams.is_empty() {
            let mut err_message = Message::new();
            err_message.set_id(request.id());
            err_message.set_message_type(MessageType::Response);
            err_message.set_op_code(OpCode::Query);
            err_message.set_response_code(ResponseCode::ServFail);
            return response_handle
                .send_response(response.build(
                    *err_message.header(),
                    err_message.answers(),
                    err_message.name_servers(),
                    err_message.name_servers(),
                    err_message.additionals(),
                ))
                .await
                .unwrap();
        }

        for q in request.queries() {
            let cache_key = (q.name().to_lowercase(), q.query_type());

            if self.config.cache.enabled
                && let Some(mut cached_response) = self.cache.get(&cache_key)
            {
                tracing::debug!("Cache hit for {:?}", cache_key);
                cached_response.set_id(request.id());
                message = cached_response;
                break;
            }

            message.set_response_code(ResponseCode::NXDomain);
            if let Some(mut resp_msg) =
                forward_to_upstream(q.name().to_lowercase(), q.query_type(), &upstreams).await
            {
                resp_msg.set_id(request.id());
                message = resp_msg.clone();
                if self.config.cache.enabled {
                    self.cache.insert(cache_key, resp_msg);
                }
                break;
            } else {
                message.set_response_code(ResponseCode::ServFail);
            }
        }
        let response_message = response.build(
            *message.header(),
            message.answers(),
            message.name_servers(),
            message.name_servers(),
            message.additionals(),
        );

        match response_handle.send_response(response_message).await {
            Ok(response_info) => response_info,
            Err(e) => {
                tracing::error!("Error sending response to {}: {}", request.src(), e);
                let mut header = *request.header();
                header.set_response_code(ResponseCode::ServFail);
                ResponseInfo::from(header)
            }
        }
    }
}
