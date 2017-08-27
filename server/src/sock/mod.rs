//! sock
//!
//! Sock is a websocket server.
//! Sister server to `web`, which is the static file server.
use ws;
use time;
use std::fmt;
use uuid::Uuid;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::{HashMap};

mod msg;
mod cookie;
mod authorizer;

use self::cookie::{parse_cookies,put_cookie};
use game::{Item,Action,Player,AnonymousPlayer};
use self::authorizer::{AuthorizesTicket,DumbTicketStamper};
use self::msg::{ActionMsg,parse};

#[derive(Debug, Clone)]
pub struct Connection {
    uuid   : Uuid,
    player : Player,
}

impl Connection {
    fn new(u: Uuid, p: Player) -> Connection {
        Connection {
            uuid  : u,
            player: p
        }
    }
}

impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "conn{{{}}}", self.uuid)
    }
}

pub type ConnectionMap = Rc<RefCell<HashMap<Uuid, Connection>>>;

struct WSServer<T> {
    out: ws::Sender,
    connections: ConnectionMap,
    authorizer: T,
}

impl ws::Handler for WSServer<DumbTicketStamper> {
    fn on_request(&mut self, req: &ws::Request) -> ws::Result<ws::Response> {
        let mut resp = ws::Response::from_request(req).unwrap();

        let mut conn_map = self.connections.borrow_mut();

        println!("ws:req[{}]", time::precise_time_ns());

        let cookies : HashMap<String, String> = parse_cookies(req);

        let cookie_existed = cookies.contains_key("bzwf_anon_wstx");
        let ticket = cookies.get("bzwf_anon_wstx")
            .and_then(|uuid_string| Uuid::parse_str(uuid_string.as_str()).ok())
            .unwrap_or_else(|| {
                println!("ws:req[{}]: no bzwf_anon_wstx cookie found", time::precise_time_ns());
                Uuid::new_v4()
            });

        let users_count = conn_map.len();

        if conn_map.contains_key(&ticket) {
            println!("ws:req[{}]: reconnect: uuid {}", time::precise_time_ns(), ticket);
            println!("{} connected users", users_count);
        } else {
            println!("ws:req[{}]: new connection with uuid {}", time::precise_time_ns(), ticket);

            let cookie_op_str = if cookie_existed {
                "replacing persistence cookie"
            } else {
                "creating persistence cookie"
            };

            println!("ws:req[{}]: {}: bzwf_anon_wstx={}", time::precise_time_ns(), cookie_op_str, ticket);
            println!("{} connected users", users_count + 1);
            put_cookie(String::from("bzwf_anon_wstx"), ticket.to_string(), &mut resp);
            conn_map.insert(ticket, Connection::new(ticket, AnonymousPlayer::new()));
        }

        Ok(resp)
    }

    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
        println!("ws:rcv[{}]: user with uuid {} sent ws msg {}",
                 time::precise_time_ns(),
                 "[[[ we don't have uuids in ws yet ]]]",
                 msg);

        let _ = parse(msg)
            .and_then(|msg_cell: ActionMsg| {
                let ref action: Action = msg_cell.val;
                let ref ticket: Uuid   = msg_cell.next.val;
                self.authorizer
                    .authorize_ticket(*ticket)
                    .map(|_| {
                        let _ = self.out.send(format!("gotcha, you want to {:?}", action));
                    })
            })
            .map_err(|err: String| {
                println!("err {}", err);
                let _ = self.out.send(err);
            });

        Ok(())
    }

    fn on_open(&mut self, _: ws::Handshake) -> ws::Result<()> {
        // TODO
        println!("ws:opn[{}]: user with uuid {} opened ws cxn",
                 time::precise_time_ns(),
                 "[[[ we don't have uuids in ws yet ]]]");
        Ok(())
    }

    fn on_close(&mut self, code: ws::CloseCode, reason: &str) {
        println!("ws:cls[{}]: user with uuid {} closed ws cxn\n\tCode [{:?}] reason: {}",
                 time::precise_time_ns(),
                 "[[[ we don't have uuids in ws yet ]]]",
                 code,
                 reason);
        // TODO
    }

    fn on_error(&mut self, err: ws::Error) {
        println!("ws:err[{}]: user with uuid {} got error {}",
                 time::precise_time_ns(),
                 "[[[ we don't have uuids in ws yet ]]]",
                 err);
        // TODO
    }
}

pub fn server (domain: String, port: i32) -> () {
    let connections: ConnectionMap = Rc::new(RefCell::new(HashMap::new()));
    let addr: String = format!("{}:{}", domain, port);
    println!("WebSockets server listening on {}", addr);
    ws::listen(addr, |out| WSServer {
        out: out,
        connections: connections.clone(),
        authorizer: DumbTicketStamper::new(connections.clone()),
    }).unwrap();
}
