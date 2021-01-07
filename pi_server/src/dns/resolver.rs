use once_cell::sync::OnceCell;
use trust_dns_proto::rr::{IntoName, RecordType};
use trust_dns_proto::xfer::DnsRequestOptions;
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::error::ResolveError;
use trust_dns_resolver::lookup::Lookup;
use trust_dns_resolver::TokioAsyncResolver;

const RESOLVER: OnceCell<TokioAsyncResolver> = OnceCell::new();

pub async fn dns_lookup(name: String, record_type: RecordType) -> Result<Lookup, ResolveError> {
    let result = RESOLVER
        .get_or_init(|| {
            TokioAsyncResolver::tokio(ResolverConfig::cloudflare_tls(), ResolverOpts::default())
                .unwrap()
        })
        .lookup(name, record_type, DnsRequestOptions::default())
        .await;
    // Do processing
    return result;
}
