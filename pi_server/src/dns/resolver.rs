use std::sync::{Arc, RwLock};
use std::time::Instant;

use tokio::macros::support::{Future, Pin};
use trust_dns_proto::op::ResponseCode;
use trust_dns_proto::rr::dnssec::SupportedAlgorithms;
use trust_dns_proto::rr::{Name, Record, RecordType};
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::lookup::Lookup as ResolverLookup;
use trust_dns_resolver::TokioAsyncResolver;
use trust_dns_server::authority::{
    Authority, AuthorityObject, LookupError, LookupObject, MessageRequest, UpdateResult, ZoneType,
};
use trust_dns_server::client::op::LowerQuery;
use trust_dns_server::client::rr::LowerName;

pub struct CloudFrontAuthority {
    origin: LowerName,
    resolver: TokioAsyncResolver,
}

impl CloudFrontAuthority {
    pub fn default() -> anyhow::Result<Self> {
        Ok(CloudFrontAuthority {
            origin: Name::root().into(),
            resolver: TokioAsyncResolver::tokio(
                ResolverConfig::cloudflare_tls(),
                ResolverOpts::default(),
            )?,
        })
    }

    pub fn into_authority_obj(self) -> Box<dyn AuthorityObject> {
        Box::new(Arc::new(RwLock::new(self)))
    }
}

impl Authority for CloudFrontAuthority {
    type Lookup = ForwardLookup;
    type LookupFuture = Pin<Box<dyn Future<Output = Result<Self::Lookup, LookupError>> + Send>>;

    fn zone_type(&self) -> ZoneType {
        ZoneType::Forward
    }

    fn is_axfr_allowed(&self) -> bool {
        false
    }

    fn update(&mut self, update: &MessageRequest) -> UpdateResult<bool> {
        log::warn!("Updated request received: {:?}", update);
        Err(ResponseCode::NotImp)
    }

    fn origin(&self) -> &LowerName {
        &self.origin
    }

    fn lookup(
        &self,
        name: &LowerName,
        rtype: RecordType,
        _is_secure: bool,
        _supported_algorithms: SupportedAlgorithms,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Lookup, LookupError>> + Send>> {
        let start = Instant::now();
        log::debug!("Lookup request: {}/{}", name, rtype);

        let lookup_request = self
            .resolver
            .lookup(name.clone(), rtype, Default::default());

        let name = name.clone();
        Box::pin(async move {
            let result = lookup_request
                .await
                .map(|lookup| ForwardLookup(lookup))
                .map_err(|e| e.into());
            match &result {
                Ok(res) => log::info!(
                    "Lookup request success [{} ms] {}[{}]: {:?}",
                    start.elapsed().as_millis(),
                    name,
                    rtype,
                    res.0
                ),
                Err(failure) => {
                    log::error!(
                        "Lookup request failed [{} ms] {}[{}]: {:?}",
                        start.elapsed().as_millis(),
                        name,
                        rtype,
                        failure
                    )
                }
            };
            result
        })
    }

    fn search(
        &self,
        query: &LowerQuery,
        is_secure: bool,
        supported_algorithms: SupportedAlgorithms,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Lookup, LookupError>> + Send>> {
        self.lookup(
            query.name(),
            query.query_type(),
            is_secure,
            supported_algorithms,
        )
    }

    fn get_nsec_records(
        &self,
        _name: &LowerName,
        _is_secure: bool,
        _supported_algorithms: SupportedAlgorithms,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Lookup, LookupError>> + Send>> {
        Box::pin(async {
            Err(LookupError::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Getting NSEC records is unimplemented for the forwarder",
            )))
        })
    }
}

pub struct ForwardLookup(ResolverLookup);

impl LookupObject for ForwardLookup {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Record> + Send + 'a> {
        Box::new(self.0.record_iter())
    }

    fn take_additionals(&mut self) -> Option<Box<dyn LookupObject>> {
        None
    }
}
