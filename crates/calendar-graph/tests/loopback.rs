use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use calendar_graph::{LoopbackListener, RedirectParams, SignInError};

const PATIENT: Duration = Duration::from_secs(5);
const IMPATIENT: Duration = Duration::from_millis(300);

fn browser_requests(listener: &LoopbackListener, target: &str) -> JoinHandle<String> {
    let address = listener.redirect_uri().socket_addrs(|| None).unwrap()[0];
    let request = format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
    thread::spawn(move || {
        let mut stream = TcpStream::connect(address).unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut page = String::new();
        stream.read_to_string(&mut page).unwrap();
        page
    })
}

#[test]
fn the_listener_answers_only_on_the_ipv4_loopback_address() {
    let listener = LoopbackListener::bind().unwrap();

    let redirect = listener.redirect_uri();

    assert_eq!(redirect.scheme(), "http");
    assert_eq!(redirect.host_str(), Some("127.0.0.1"));
    assert_ne!(redirect.port(), Some(0));
    assert_eq!(redirect.path(), "/");
}

#[test]
fn a_redirect_carrying_a_code_yields_the_code_and_its_state() {
    let listener = LoopbackListener::bind().unwrap();
    let browser = browser_requests(&listener, "/?code=auth-code&state=expected-state");

    let params = listener.wait_for_redirect(PATIENT).unwrap();

    browser.join().unwrap();
    let RedirectParams::Code { code, state } = params else {
        panic!("expected a code, got {params:?}");
    };
    assert_eq!(code.expose(), "auth-code");
    assert_eq!(state, "expected-state");
}

#[test]
fn the_browser_is_told_it_can_close_the_tab() {
    let listener = LoopbackListener::bind().unwrap();
    let browser = browser_requests(&listener, "/?code=auth-code&state=expected-state");

    listener.wait_for_redirect(PATIENT).unwrap();

    let page = browser.join().unwrap();
    assert!(page.starts_with("HTTP/1.1 200 OK"));
    assert!(page.contains("You can close this tab"));
}

#[test]
fn a_redirect_carrying_an_error_yields_microsofts_refusal() {
    let listener = LoopbackListener::bind().unwrap();
    let browser = browser_requests(
        &listener,
        "/?error=access_denied&error_description=AADSTS65004%3A+User+declined&state=s",
    );

    let params = listener.wait_for_redirect(PATIENT).unwrap();

    browser.join().unwrap();
    assert_eq!(
        params,
        RedirectParams::Refused {
            error: "access_denied".to_owned(),
            description: "AADSTS65004: User declined".to_owned(),
        }
    );
}

#[test]
fn a_redirect_with_neither_a_code_nor_an_error_is_malformed() {
    let listener = LoopbackListener::bind().unwrap();
    let browser = browser_requests(&listener, "/favicon.ico");

    let outcome = listener.wait_for_redirect(PATIENT);

    browser.join().unwrap();
    assert!(matches!(outcome, Err(SignInError::Malformed)));
}

#[test]
fn the_listener_refuses_a_second_connection_after_the_first() {
    let listener = LoopbackListener::bind().unwrap();
    let address = listener.redirect_uri().socket_addrs(|| None).unwrap()[0];
    let browser = browser_requests(&listener, "/?code=auth-code&state=expected-state");

    listener.wait_for_redirect(PATIENT).unwrap();

    browser.join().unwrap();
    assert!(TcpStream::connect(address).is_err());
}

#[test]
fn a_redirect_that_never_arrives_times_out() {
    let listener = LoopbackListener::bind().unwrap();

    let outcome = listener.wait_for_redirect(IMPATIENT);

    assert!(matches!(outcome, Err(SignInError::TimedOut)));
}
