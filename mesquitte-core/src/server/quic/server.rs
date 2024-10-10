use std::{net::SocketAddr, sync::Arc};

use s2n_quic::{provider::tls, Server};

use crate::{
    server::{process_client, state::GlobalState},
    store::queue::Queue,
};

use super::Error;

pub struct QuicServer<Q>
where
    Q: Queue,
{
    inner: Server,
    global: Arc<GlobalState<Q>>,
}

impl<Q> QuicServer<Q>
where
    Q: Queue + Send + 'static,
{
    pub fn bind<T: tls::TryInto>(
        addr: SocketAddr,
        tls: T,
        global: Arc<GlobalState<Q>>,
    ) -> Result<Self, Error>
    where
        Error: From<<T as tls::TryInto>::Error>,
    {
        let server = Server::builder().with_tls(tls)?.with_io(addr)?.start()?;
        Ok(QuicServer {
            inner: server,
            global,
        })
    }

    pub async fn accept(mut self) -> Result<(), Error> {
        while let Some(mut connection) = self.inner.accept().await {
            let g = self.global.clone();
            tokio::spawn(async move {
                while let Ok(Some(stream)) = connection.accept_bidirectional_stream().await {
                    process_client(stream, g.clone()).await;
                }
            });
        }
        Ok(())
    }
}
