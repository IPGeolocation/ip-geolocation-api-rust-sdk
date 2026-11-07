use ipgeolocation::{
    BulkLookupIpGeolocationRequest, IpGeolocationClient, IpGeolocationClientConfig,
    IpGeolocationError, LookupIpGeolocationRequest,
};
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

const KEY: &str = "sample +/marker";

#[derive(Default)]
struct Request {
    head: String,
    body: Vec<u8>,
}

fn read_request(stream: &TcpStream) -> Request {
    let mut reader = BufReader::new(stream);
    let mut request = Request::default();
    let mut length = 0;
    loop {
        let mut line = String::new();
        assert!(reader.read_line(&mut line).unwrap() > 0);
        if line == "\r\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("Content-Length") {
                length = value.trim().parse().unwrap();
            }
        }
        request.head.push_str(&line);
    }
    request.body.resize(length, 0);
    reader.read_exact(&mut request.body).unwrap();
    request
}

fn serve(responses: Vec<Vec<u8>>) -> (IpGeolocationClient, thread::JoinHandle<Vec<Request>>) {
    serve_with_key(responses, KEY)
}

fn serve_with_key(
    responses: Vec<Vec<u8>>,
    key: &str,
) -> (IpGeolocationClient, thread::JoinHandle<Vec<Request>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = IpGeolocationClient::new(IpGeolocationClientConfig {
        base_url: format!("http://{}", listener.local_addr().unwrap()),
        api_key: Some(format!(" {key} ")),
        request_timeout_ms: 3_000,
        ..Default::default()
    })
    .unwrap();
    let join = thread::spawn(move || {
        responses
            .into_iter()
            .map(|reply| {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let request = read_request(&stream);
                stream.write_all(&reply).unwrap();
                request
            })
            .collect()
    });
    (client, join)
}

fn response(status: &str, headers: &str, body: &[u8]) -> Vec<u8> {
    let mut reply = format!("HTTP/1.1 {status}\r\nConnection: close\r\n{headers}\r\n").into_bytes();
    reply.extend_from_slice(body);
    reply
}

fn ok(body: &str) -> Vec<u8> {
    response("200 OK", "", body.as_bytes())
}

fn single() -> LookupIpGeolocationRequest {
    LookupIpGeolocationRequest {
        ip: Some("8.8.8.8".into()),
        lang: Some("ar".into()),
        headers: [("X-Custom".into(), "header-marker".into())].into(),
        ..Default::default()
    }
}

fn bulk() -> BulkLookupIpGeolocationRequest {
    BulkLookupIpGeolocationRequest {
        ips: vec!["8.8.8.8".into(), "8.8.8.8".into()],
        lang: Some("ar".into()),
        headers: single().headers,
        ..Default::default()
    }
}

#[test]
fn redirects_do_not_send_lookup_data_to_another_origin() {
    let target = TcpListener::bind("127.0.0.1:0").unwrap();
    let location = format!(
        "Location: http://{}/target\r\n",
        target.local_addr().unwrap()
    );
    let (client, server) = serve(vec![
        response("302 Found", &location, b""),
        response("307 Temporary Redirect", &location, b""),
    ]);
    for (error, status) in [
        (
            client.lookup_ip_geolocation_raw(&single()).unwrap_err(),
            302,
        ),
        (
            client.bulk_lookup_ip_geolocation_raw(&bulk()).unwrap_err(),
            307,
        ),
    ] {
        assert!(matches!(error, IpGeolocationError::Api(e) if e.status_code() == status));
    }
    let requests = server.join().unwrap();
    assert!(requests.iter().all(
        |r| r.head.to_lowercase().contains("x-custom: header-marker") && r.head.contains("lang=ar")
    ));
    assert_eq!(requests[1].body, br#"{"ips":["8.8.8.8","8.8.8.8"]}"#);
    target.set_nonblocking(true).unwrap();
    assert_eq!(target.accept().unwrap_err().kind(), ErrorKind::WouldBlock);
}

#[test]
fn same_origin_redirect_keeps_bulk_body_and_headers_without_a_referrer() {
    let (client, server) = serve(vec![
        response("307 Temporary Redirect", "Location: /final\r\n", b""),
        ok("[{},{}]"),
    ]);
    assert_eq!(
        client
            .bulk_lookup_ip_geolocation(&bulk())
            .unwrap()
            .data
            .len(),
        2
    );
    let requests = server.join().unwrap();
    assert!(requests[1].head.starts_with("POST /final "));
    assert!(requests[1]
        .head
        .to_lowercase()
        .contains("x-custom: header-marker"));
    assert!(!requests[1].head.to_lowercase().contains("referer:"));
    assert_eq!(requests[0].body, requests[1].body);
}

#[test]
fn unreadable_http_errors_keep_their_status() {
    let (client, server) = serve(vec![
        response("429 Too Many Requests", "Content-Length: 200\r\n", b"short"),
        response("401 Unauthorized", "", &[0xff]),
        response("200 OK", "", &[0xff]),
    ]);
    for (error, status) in [
        (client.lookup_ip_geolocation(&single()).unwrap_err(), 429),
        (
            client
                .bulk_lookup_ip_geolocation_detailed(&bulk())
                .unwrap_err(),
            401,
        ),
    ] {
        assert!(matches!(error, IpGeolocationError::Api(e) if e.status_code() == status));
    }
    assert!(matches!(
        client.lookup_ip_geolocation_raw(&single()),
        Err(IpGeolocationError::Serialization { .. })
    ));
    server.join().unwrap();
}

#[test]
fn typed_shapes_and_error_details_are_checked_but_raw_bodies_are_preserved() {
    let bad = r#"{"network":{"is_anycast":"sample +/marker"}}"#;
    let mixed =
        r#"[{"location":{"city":"دبي"}},{"error":{"message":"invalid sample+%2B%2Fmarker"}}]"#;
    let (client, server) = serve(vec![
        ok("[]"),
        ok("[{}]"),
        ok("[{},{},{}]"),
        ok("[[],{}]"),
        ok(bad),
        ok(mixed),
        ok(mixed),
        ok(mixed),
        ok(r#"{"ip":"first","ip":"second"}"#),
        ok(" {} "),
    ]);
    for error in [
        client.lookup_ip_geolocation(&single()).unwrap_err(),
        client.bulk_lookup_ip_geolocation(&bulk()).unwrap_err(),
        client
            .bulk_lookup_ip_geolocation_detailed(&bulk())
            .unwrap_err(),
        client.bulk_lookup_ip_geolocation(&bulk()).unwrap_err(),
    ] {
        assert!(matches!(error, IpGeolocationError::Serialization { .. }));
    }
    let error = client
        .lookup_ip_geolocation_detailed(&single())
        .unwrap_err();
    assert!(matches!(error, IpGeolocationError::Serialization { .. }));
    assert!(!format!("{error:?}").contains(KEY));
    assert!(!std::error::Error::source(&error)
        .unwrap()
        .to_string()
        .contains(KEY));
    let result = client.bulk_lookup_ip_geolocation(&bulk()).unwrap();
    assert_eq!(
        result.data[0]
            .data
            .as_ref()
            .unwrap()
            .location
            .as_ref()
            .unwrap()
            .city
            .as_deref(),
        Some("دبي")
    );
    assert_eq!(
        result.data[1].error.as_ref().unwrap().message.as_deref(),
        Some("[REDACTED]")
    );
    let detailed = client.bulk_lookup_ip_geolocation_detailed(&bulk()).unwrap();
    assert_eq!(
        detailed.data[1].as_ref().unwrap_err().message.as_deref(),
        Some("[REDACTED]")
    );
    assert_eq!(
        client.bulk_lookup_ip_geolocation_raw(&bulk()).unwrap().data,
        mixed
    );
    assert!(matches!(
        client.lookup_ip_geolocation(&single()),
        Err(IpGeolocationError::Serialization { .. })
    ));
    assert!(client
        .lookup_ip_geolocation(&single())
        .unwrap()
        .data
        .ip
        .is_none());
    server.join().unwrap();
}

#[test]
fn api_errors_hide_keys_in_all_lookup_methods() {
    let reply = response(
        "403 Forbidden",
        "",
        br#"{"message":"invalid sample +/marker"}"#,
    );
    let (client, server) = serve(vec![reply; 6]);
    for error in [
        client.lookup_ip_geolocation(&single()).unwrap_err(),
        client
            .lookup_ip_geolocation_detailed(&single())
            .unwrap_err(),
        client.lookup_ip_geolocation_raw(&single()).unwrap_err(),
        client.bulk_lookup_ip_geolocation(&bulk()).unwrap_err(),
        client
            .bulk_lookup_ip_geolocation_detailed(&bulk())
            .unwrap_err(),
        client.bulk_lookup_ip_geolocation_raw(&bulk()).unwrap_err(),
    ] {
        assert!(!format!("{error:?}").contains(KEY));
        assert!(
            matches!(error, IpGeolocationError::Api(e) if e.status_code() == 403 && e.message() == "invalid [REDACTED]")
        );
    }
    server.join().unwrap();
}

#[test]
fn connection_errors_do_not_retain_the_key_or_url() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let client = IpGeolocationClient::new(IpGeolocationClientConfig {
        base_url: url,
        api_key: Some("local-test-marker".into()),
        ..Default::default()
    })
    .unwrap();
    let error = client.lookup_ip_geolocation_raw(&single()).unwrap_err();
    assert!(matches!(error, IpGeolocationError::Transport { .. }));
    assert!(!format!("{error:?}").contains("local-test-marker"));
    let mut cause = std::error::Error::source(&error);
    while let Some(current) = cause {
        assert!(!format!("{current:?} {current}").contains("local-test-marker"));
        cause = current.source();
    }
}

#[test]
fn detailed_lookups_keep_location_network_and_security_fields() {
    let data = serde_json::json!({
        "ip":"8.8.8.8",
        "location":{"country_name":"ألمانيا","continent_code":"EU","continent_name":"Europe",
            "country_code3":"DEU","country_name_official":"Germany","country_capital":"Berlin",
            "state_code":"DE-BE","district":"Mitte","locality":"","zipcode":"10115",
            "country_flag":"flag.png","geoname_id":"2950159","country_emoji":"🇩🇪",
            "accuracy_radius":"5","dma_code":"","confidence":"high","is_eu":true},
        "network":{"is_cdn":true,"cdn_provider_name":"example"},
        "security":{"proxy_days_seen_7d":1,"proxy_days_seen_30d":12,"proxy_provider_count_30d":3,
            "bot_type":"crawler","is_known_good_bot":true,"bot_operator_name":"example",
            "bot_confidence_score":98.5,"bot_last_seen":"2026-01-01","is_corporate_gateway":false,
            "corporate_gateway_provider_name":"","corporate_gateway_type":""}
    });
    let (client, server) = serve(vec![
        ok(&data.to_string()),
        ok(&serde_json::json!([data, {"message":"invalid IP"}]).to_string()),
    ]);
    let one = client.lookup_ip_geolocation_detailed(&single()).unwrap();
    let many = client.bulk_lookup_ip_geolocation_detailed(&bulk()).unwrap();
    assert_eq!(many.data[0].as_ref().unwrap(), &one.data);
    assert_eq!(
        many.data[1].as_ref().unwrap_err().message.as_deref(),
        Some("invalid IP")
    );
    let typed = serde_json::to_value(&one.data).unwrap();
    for group in ["location", "network", "security"] {
        for (field, value) in data[group].as_object().unwrap() {
            assert_eq!(&typed[group][field], value, "{group}.{field}");
        }
    }
    assert!(one.data.asn.is_none());
    assert!(server
        .join()
        .unwrap()
        .iter()
        .all(|request| request.head.contains("lang=ar")));
}

#[test]
fn bulk_handles_fifty_thousand_results_in_order() {
    let n = 50_000;
    let ips = (0..n)
        .map(|i| format!("192.0.{}.{}", i / 256, i % 256))
        .collect::<Vec<_>>();
    let body = serde_json::to_string(
        &ips.iter()
            .map(|ip| serde_json::json!({"ip":ip}))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let (client, server) = serve(vec![ok(&body)]);
    let request = BulkLookupIpGeolocationRequest {
        ips,
        ..Default::default()
    };
    let response = client.bulk_lookup_ip_geolocation(&request).unwrap();
    assert_eq!(response.data.len(), n);
    assert!(response.data.iter().zip(&request.ips).all(|(row, ip)| row
        .data
        .as_ref()
        .unwrap()
        .ip
        .as_ref()
        == Some(ip)));
    server.join().unwrap();
}

#[test]
fn escaped_keys_are_hidden_and_unrelated_errors_stay_readable() {
    for (key, body, expected) in [
        ("x+y/z", r#"{"reason":"bad x+y\/z"}"#, "[REDACTED]"),
        (
            "x+y/z",
            r#"{"message":"invalid x%252By%252Fz"}"#,
            "[REDACTED]",
        ),
        (
            r"example\new&marker",
            r"<message>bad example\new&amp;marker</message>",
            "[REDACTED]",
        ),
        (
            r#"example&amp;"marker"#,
            r#"{"reason":"bad example&amp;\"marker"}"#,
            "[REDACTED]",
        ),
        (
            "test-fixture",
            "<message>bad test-&#00000000000000000102;ixture</message>",
            "[REDACTED]",
        ),
        ("x&y", "<message>Bad x&amp;y</message>", "[REDACTED]"),
        ("x&y", "<message>Bad x&#38;y</message>", "[REDACTED]"),
        ("x&y", "<message>Bad x&#x26;y</message>", "[REDACTED]"),
        ("x💡y", r#"{"reason":"bad x\ud83d\udca1y"}"#, "[REDACTED]"),
        (
            "unrelated",
            r"File C:\users\cache was unavailable",
            r"File C:\users\cache was unavailable",
        ),
    ] {
        let (client, server) =
            serve_with_key(vec![response("403 Forbidden", "", body.as_bytes())], key);
        let error = client.lookup_ip_geolocation_raw(&single()).unwrap_err();
        let IpGeolocationError::Api(error) = error else {
            panic!("expected an API error");
        };
        server.join().unwrap();
        assert_eq!(error.status_code(), 403);
        assert_eq!(error.message(), expected, "{body}");
    }
}
