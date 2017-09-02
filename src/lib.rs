//! Extract different env variables.

#[cfg(test)]
#[macro_use]
extern crate lazy_static;
extern crate url;

use std::env::var_os;
use url::Url;

fn is_no_proxy(url: &Url) -> bool {
	let maybe_no_proxy = var_os("no_proxy")
		.or_else(|| var_os("NO_PROXY"))
		.map(|v| v.to_str().unwrap_or("").to_string());

	if let Some(no_proxy) = maybe_no_proxy {
		if no_proxy == "*" {
			return true;
		}
		if let Some(host) = url.host_str() {
			for elem in no_proxy.split(|c| c == ',' || c == ' ') {
				if host.ends_with(elem) {
					return true;
				}
			}
		}
	}
	false
}

/// Extract proxy parameters for a URL by examining the environment variables.
///
/// Most environment variables described here can be defined either with an all-lowercase or an
/// all-uppercase name. If both versions are defined, the all-lowercase name takes precedence
///
/// If __no_proxy__ is defined, check the host part of the URL against its components and return
/// `None` if there is any match. The value of __no_proxy__ should be a space- or comma-separated
/// list of host/domain names or IP addresses for which no proxying should be done, or a single
/// '&#8239;__*__&#8239;' (asterisk) which means that proxying is disabled for all hosts.
///
/// If the port is not explicitly defined in the proxy URL, the value 8080 is used.
pub fn for_url(s: &str) -> Option<String> {
	let url = if let Ok(u) = Url::parse(s) {
		u
	} else {
		return None;
	};

	if is_no_proxy(&url) {
		return None;
	}

	let maybe_https_proxy = var_os("https_proxy")
		.or_else(|| var_os("HTTPS_PROXY"))
		.map(|v| v.to_str().unwrap_or("").to_string());
	let maybe_ftp_proxy = var_os("ftp_proxy")
		.or_else(|| var_os("FTP_PROXY"))
		.map(|v| v.to_str().unwrap_or("").to_string());
	let maybe_http_proxy = var_os("http_proxy")
		.or_else(|| var_os("HTTP_PROXY"))
		.map(|v| v.to_str().unwrap_or("").to_string());
	let maybe_all_proxy = var_os("all_proxy")
		.or_else(|| var_os("ALL_PROXY"))
		.map(|v| v.to_str().unwrap_or("").to_string());

	if let Some(url_value) = match url.scheme() {
		"https" => maybe_https_proxy.or(maybe_http_proxy.or(maybe_all_proxy)),
		"http" => maybe_http_proxy.or(maybe_all_proxy),
		"ftp" => maybe_ftp_proxy.or(maybe_http_proxy.or(maybe_all_proxy)),
		_ => maybe_all_proxy,
	} {
		if let Ok(mut proxy_url) = Url::parse(&url_value) {
			if proxy_url.host_str().is_some() {
				if proxy_url.port().is_some() {
					return Some(url_value);
				} else {
					if proxy_url.set_port(Some(8080)).is_ok() {
						return Some(proxy_url.as_str().to_string());
					}
				}
			}
		}
	}
	None
}

#[cfg(test)]
mod tests {
	use std::env::{remove_var, set_var};
	use std::sync::Mutex;
	use super::*;

	// environment is per-process, and we need it stable per-thread,
	// hence locking
	lazy_static! {
	static ref LOCK: Mutex<()> = Mutex::new(());
	}

	fn scrub_env() {
		remove_var("http_proxy");
		remove_var("https_proxy");
		remove_var("HTTPS_PROXY");
		remove_var("ftp_proxy");
		remove_var("FTP_PROXY");
		remove_var("all_proxy");
		remove_var("ALL_PROXY");
		remove_var("no_proxy");
		remove_var("NO_PROXY");
	}

	#[test]
	fn no_proxy_simple_name() {
		let _l = LOCK.lock();
		scrub_env();
		set_var("no_proxy", "example.org");
		set_var("http_proxy", "http://proxy.example.com:8080");
		assert!(for_url("http://example.org").is_none());
	}

	#[test]
	fn no_proxy_global() {
		let _l = LOCK.lock();
		scrub_env();
		set_var("no_proxy", "*");
		set_var("http_proxy", "http://proxy.example.com:8080");
		assert!(for_url("http://example.org").is_none());
	}

	#[test]
	fn no_proxy_subdomain() {
		let _l = LOCK.lock();
		scrub_env();
		set_var("no_proxy", "example.org");
		set_var("http_proxy", "http://proxy.example.com:8080");
		assert!(for_url("http://www.example.org").is_none());
	}

	#[test]
	fn no_proxy_subdomain_dot() {
		let _l = LOCK.lock();
		scrub_env();
		set_var("no_proxy", "example.org");
		set_var("http_proxy", "http://proxy.example.com:8080");
		assert!(for_url("http://www.example.org").is_none());
	}

	#[test]
	fn no_proxy_multiple_list() {
		let _l = LOCK.lock();
		scrub_env();
		set_var(
			"no_proxy",
			"www.example.org,www.example1.org,www.example.org",
		);
		set_var("http_proxy", "http://proxy.example.com:8080");
		assert!(for_url("http://www.example.org").is_none());
	}

	#[test]
	fn http_proxy_specific() {
		let _l = LOCK.lock();
		scrub_env();
		set_var("http_proxy", "http://proxy.example.com:8080");
		set_var("all_proxy", "http://proxy.example.org:8081");
		assert_eq!(
			for_url("http://www.example.org"),
			Some(("http://proxy.example.com:8080".to_string()))
		);
	}

	#[test]
	fn http_proxy_fallback() {
		let _l = LOCK.lock();
		scrub_env();
		set_var("ALL_PROXY", "http://proxy.example.com:8080");
		assert_eq!(
			for_url("http://www.example.org"),
			Some(("http://proxy.example.com:8080".to_string()))
		);
		set_var("all_proxy", "http://proxy.example.org:8081");
		assert_eq!(
			for_url("http://www.example.org"),
			Some(("http://proxy.example.org:8081".to_string()))
		);
	}

	#[test]
	fn https_proxy_specific() {
		let _l = LOCK.lock();
		scrub_env();
		set_var("HTTPS_PROXY", "http://proxy.example.com:8080");
		set_var("http_proxy", "http://proxy.example.org:8081");
		set_var("all_proxy", "http://proxy.example.org:8081");
		assert_eq!(
			Some(("http://proxy.example.com:8080".to_string())),
			for_url("https://www.example.org")
		);
		set_var("https_proxy", "http://proxy.example.com:8081");
		assert_eq!(
			for_url("https://www.example.org"),
			Some(("http://proxy.example.com:8081".to_string()))
		);
	}

	#[test]
	fn https_proxy_fallback() {
		let _l = LOCK.lock();
		scrub_env();
		set_var("http_proxy", "http://proxy.example.com:8080");
		set_var("ALL_PROXY", "http://proxy.example.org:8081");
		assert_eq!(
			for_url("https://www.example.org"),
			Some(("http://proxy.example.com:8080".to_string()))
		);
		remove_var("http_proxy");
		assert_eq!(
			for_url("https://www.example.org"),
			Some(("http://proxy.example.org:8081".to_string()))
		);
		set_var("all_proxy", "http://proxy.example.org:8082");
		assert_eq!(
			for_url("https://www.example.org"),
			Some(("http://proxy.example.org:8082".to_string()))
		);
	}

	#[test]
	fn ftp_proxy_specific() {
		let _l = LOCK.lock();
		scrub_env();
		set_var("FTP_PROXY", "http://proxy.example.com:8080");
		set_var("http_proxy", "http://proxy.example.org:8081");
		set_var("all_proxy", "http://proxy.example.org:8081");
		assert_eq!(
			for_url("ftp://www.example.org"),
			Some(("http://proxy.example.com:8080".to_string()))
		);
		set_var("ftp_proxy", "http://proxy.example.com:8081");
		assert_eq!(
			for_url("ftp://www.example.org"),
			Some(("http://proxy.example.com:8081".to_string()))
		);
	}

	#[test]
	fn ftp_proxy_fallback() {
		let _l = LOCK.lock();
		scrub_env();
		set_var("http_proxy", "http://proxy.example.com:8080");
		set_var("ALL_PROXY", "http://proxy.example.org:8081");
		assert_eq!(
			for_url("ftp://www.example.org"),
			Some(("http://proxy.example.com:8080".to_string()))
		);
		remove_var("http_proxy");
		assert_eq!(
			for_url("ftp://www.example.org"),
			Some(("http://proxy.example.org:8081".to_string()))
		);
		set_var("all_proxy", "http://proxy.example.org:8082");
		assert_eq!(
			for_url("ftp://www.example.org"),
			Some(("http://proxy.example.org:8082".to_string()))
		);
	}
}
