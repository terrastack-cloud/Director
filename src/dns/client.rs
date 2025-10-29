use core::net::SocketAddr;
use hickory_client::client::{Client, ClientHandle};
use hickory_proto::{
    op::Message,
    rr::{DNSClass, Name, RecordType},
    udp::UdpClientStream,
};

pub async fn forward_to_upstream(
    name: Name,
    qtype: RecordType,
    upstreams: &[SocketAddr],
) -> Option<Message> {
    for &upstream in upstreams {
        let udp_builder = UdpClientStream::builder(
            upstream,
            hickory_proto::runtime::TokioRuntimeProvider::default(),
        );
        let udp = udp_builder.build();
        if let Ok((mut client, bg)) = Client::connect(udp).await {
            tokio::spawn(bg);
            let qclass = DNSClass::IN;
            if let Ok(response) = client.query(name.clone(), qclass, qtype).await {
                return Some(response.into_message());
            }
        }
    }
    None
}
