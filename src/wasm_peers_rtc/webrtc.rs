use std::{collections::{HashMap, VecDeque}, rc::Rc, cell::RefCell};

use js_sys::{Reflect, Promise, Array, Object};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{WebSocket, RtcPeerConnection, RtcPeerConnectionIceEvent, RtcSessionDescriptionInit, RtcSdpType, RtcIceCandidateInit, RtcIceCandidate, RtcDataChannelEvent, RtcConfiguration};
use wasm_bindgen::prelude::*;

use super::{callback_channel::SendRecvCallbackChannel, signaling::{ServerEntry, ConnectionId, SignalingMessage, RelayMessage, SignalingDemux, SignalingDemuxRecv, SignalingClientConnection}};

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}
macro_rules! console_warn {
    ($($t:tt)*) => (warn(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    #[wasm_bindgen(js_namespace = console)]
    fn warn(s: &str);
}

pub struct WebRtcBrowser {
    websocket: SendRecvCallbackChannel,
    servers: HashMap<ConnectionId, ServerEntry>
}

fn make_rtc_peer() -> Result<RtcPeerConnection, JsValue> {
    let stun_servers = [
        "iphone-stun.strato-iphone.de:3478",
        "numb.viagenie.ca:3478",
        "stun.12connect.com:3478",
        "stun.12voip.com:3478",
        "stun.1und1.de:3478",
        "stun.3cx.com:3478",
        "stun.acrobits.cz:3478",
        "stun.actionvoip.com:3478",
        "stun.advfn.com:3478",
        "stun.altar.com.pl:3478",
        "stun.antisip.com:3478",
        "stun.avigora.fr:3478",
        "stun.bluesip.net:3478",
        "stun.cablenet-as.net:3478",
        "stun.callromania.ro:3478",
        "stun.callwithus.com:3478",
        "stun.cheapvoip.com:3478",
        "stun.cloopen.com:3478",
        "stun.commpeak.com:3478",
        "stun.cope.es:3478",
        "stun.counterpath.com:3478",
        "stun.counterpath.net:3478",
        "stun.dcalling.de:3478",
        "stun.demos.ru:3478",
        "stun.dus.net:3478",
        "stun.easycall.pl:3478",
        "stun.easyvoip.com:3478",
        "stun.ekiga.net:3478",
        "stun.epygi.com:3478",
        "stun.etoilediese.fr:3478",
        "stun.faktortel.com.au:3478",
        "stun.freecall.com:3478",
        "stun.freeswitch.org:3478",
        "stun.freevoipdeal.com:3478",
        "stun.gmx.de:3478",
        "stun.gmx.net:3478",
        "stun.halonet.pl:3478",
        "stun.hoiio.com:3478",
        "stun.hosteurope.de:3478",
        "stun.infra.net:3478",
        "stun.internetcalls.com:3478",
        "stun.intervoip.com:3478",
        "stun.ipfire.org:3478",
        "stun.ippi.fr:3478",
        "stun.ipshka.com:3478",
        "stun.it1.hr:3478",
        "stun.ivao.aero:3478",
        "stun.jumblo.com:3478",
        "stun.justvoip.com:3478",
        "stun.l.google.com:19302",
        "stun.linphone.org:3478",
        "stun.liveo.fr:3478",
        "stun.lowratevoip.com:3478",
        "stun.lundimatin.fr:3478",
        "stun.mit.de:3478",
        "stun.miwifi.com:3478",
        "stun.modulus.gr:3478",
        "stun.myvoiptraffic.com:3478",
        "stun.netappel.com:3478",
        "stun.netgsm.com.tr:3478",
        "stun.nfon.net:3478",
        "stun.nonoh.net:3478",
        "stun.nottingham.ac.uk:3478",
        "stun.ooma.com:3478",
        "stun.ozekiphone.com:3478",
        "stun.pjsip.org:3478",
        "stun.poivy.com:3478",
        "stun.powervoip.com:3478",
        "stun.ppdi.com:3478",
        "stun.qq.com:3478",
        "stun.rackco.com:3478",
        "stun.rockenstein.de:3478",
        "stun.rolmail.net:3478",
        "stun.rynga.com:3478",
        "stun.schlund.de:3478",
        "stun.sigmavoip.com:3478",
        "stun.sip.us:3478",
        "stun.sipdiscount.com:3478",
        "stun.sipgate.net:10000",
        "stun.sipgate.net:3478",
        "stun.siplogin.de:3478",
        "stun.sipnet.net:3478",
        "stun.sipnet.ru:3478",
        "stun.sippeer.dk:3478",
        "stun.siptraffic.com:3478",
        "stun.sma.de:3478",
        "stun.smartvoip.com:3478",
        "stun.smsdiscount.com:3478",
        "stun.solcon.nl:3478",
        "stun.solnet.ch:3478",
        "stun.sonetel.com:3478",
        "stun.sonetel.net:3478",
        "stun.sovtest.ru:3478",
        "stun.srce.hr:3478",
        "stun.stunprotocol.org:3478",
        "stun.t-online.de:3478",
        "stun.tel.lu:3478",
        "stun.telbo.com:3478",
        "stun.tng.de:3478",
        "stun.twt.it:3478",
        "stun.uls.co.za:3478",
        "stun.unseen.is:3478",
        "stun.usfamily.net:3478",
        "stun.viva.gr:3478",
        "stun.vivox.com:3478",
        "stun.vo.lu:3478",
        "stun.voicetrading.com:3478",
        "stun.voip.aebc.com:3478",
        "stun.voip.blackberry.com:3478",
        "stun.voip.eutelia.it:3478",
        "stun.voipblast.com:3478",
        "stun.voipbuster.com:3478",
        "stun.voipbusterpro.com:3478",
        "stun.voipcheap.co.uk:3478",
        "stun.voipcheap.com:3478",
        "stun.voipgain.com:3478",
        "stun.voipgate.com:3478",
        "stun.voipinfocenter.com:3478",
        "stun.voipplanet.nl:3478",
        "stun.voippro.com:3478",
        "stun.voipraider.com:3478",
        "stun.voipstunt.com:3478",
        "stun.voipwise.com:3478",
        "stun.voipzoom.com:3478",
        "stun.voys.nl:3478",
        "stun.voztele.com:3478",
        "stun.webcalldirect.com:3478",
        "stun.wifirst.net:3478",
        "stun.xtratelecom.es:3478",
        "stun.zadarma.com:3478",
        "stun1.faktortel.com.au:3478",
        "stun1.l.google.com:19302",
        "stun2.l.google.com:19302",
        "stun3.l.google.com:19302",
        "stun4.l.google.com:19302",
        "stun.nextcloud.com:443",
        "relay.webwormhole.io:3478"
    ];
    let mut config = RtcConfiguration::new();
    let c = config.ice_servers(&Array::from_iter(stun_servers.iter().map(|str| {
        let o = Object::new();
        Reflect::set(&o, &JsValue::from_str("urls"), &Object::from(JsValue::from_str(&format!("stun:{}", str)))).unwrap();
        o
    })));

    RtcPeerConnection::new_with_configuration(c)
}

impl WebRtcBrowser {
    pub async fn new(signaling_url: &str) -> Result<WebRtcBrowser, JsValue> {
        let websocket = WebSocket::new(signaling_url)?;
        let mut ws = SendRecvCallbackChannel::new(Box::new(websocket)).await?;
        let msg: SignalingMessage = ws.recv().await?;
        let servers = if let SignalingMessage::List { servers } = msg {
            servers
        } else {
            return Err(JsValue::from_str(&format!("Unexpected msg {:?}", msg)));
        };
        Ok(WebRtcBrowser { websocket: ws, servers })
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &ServerEntry)> {
        self.servers.iter()
    }

    pub async fn connect(mut self, server_id: ConnectionId) -> Result<WebRtcClient, JsValue> {
        if !self.servers.contains_key(&server_id) {
            return Err(JsValue::from_str(&format!("Invalid server id {}", server_id)));
        }

        let peer = make_rtc_peer()?;

        // Create data channel
        let data = peer.create_data_channel("my-data-channel");

        // Start sending ICE candidates to server
        let mut ws_cloned = self.websocket.clone();
        let server_id_cloned = server_id.clone();
        let onicecandidate_callback1 =
            Closure::<dyn FnMut(_)>::new(move |ev: RtcPeerConnectionIceEvent| {
                if let Some(candidate) = ev.candidate() {
                    let msg = SignalingMessage::Relay {
                        src: "".to_owned(),
                        dst: server_id_cloned.to_owned(),
                        data: RelayMessage::IceCandidate {
                            candidate: candidate.candidate(),
                            sdp_mid: candidate.sdp_mid(),
                            sdp_m_line_index: candidate.sdp_m_line_index(),
                        }
                    };
                    ws_cloned.send(msg).unwrap();
                }
            });
        peer.set_onicecandidate(Some(onicecandidate_callback1.as_ref().unchecked_ref()));
        onicecandidate_callback1.forget();

        // Send OFFER to server
        let offer = JsFuture::from(peer.create_offer()).await?;
        let offer_sdp = Reflect::get(&offer, &JsValue::from_str("sdp"))?
            .as_string()
            .unwrap();

        let mut offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        offer_obj.sdp(&offer_sdp);
        let sld_promise = peer.set_local_description(&offer_obj);
        JsFuture::from(sld_promise).await?;

        let msg = SignalingMessage::Relay {
            src: "".to_owned(),
            dst: server_id.to_owned(),
            data: RelayMessage::Offer(offer_sdp)
        };
        self.websocket.send(msg).unwrap();

        // Receive ANSWER from server
        let msg: SignalingMessage = self.websocket.recv().await?;
        let answer_sdp = if let SignalingMessage::Relay { data: RelayMessage::Answer(answer_sdp), .. } = msg {
            answer_sdp
        } else {
            return Err(JsValue::from_str(&format!("Unexpected msg: {:?}", msg)));
        };

        let mut answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        answer_obj.sdp(&answer_sdp);
        let srd_promise = peer.set_remote_description(&answer_obj);
        JsFuture::from(srd_promise).await?;

        // Recv ICE candidates from server
        let mut ws_cloned = self.websocket.clone();
        let pc1_clone = peer.clone();
        spawn_local(async move { // IMPORTANT: After this point, only this task may recv().
            loop {
                let msg: SignalingMessage = ws_cloned.recv().await.unwrap();
                if let SignalingMessage::Relay { data: RelayMessage::IceCandidate { candidate, sdp_mid, sdp_m_line_index }, .. } = msg {
                    let mut init = RtcIceCandidateInit::new(&candidate);
                    let sdp_mid = match &sdp_mid {
                        Some(str) => Some(str.as_str()),
                        None => None
                    };
                    init.sdp_mid(sdp_mid);
                    init.sdp_m_line_index(sdp_m_line_index);
                    let cand = RtcIceCandidate::new(&init).unwrap();
                    JsFuture::from(pc1_clone.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&cand))).await.unwrap();
                } else {
                    console_warn!("Recv unexpected: {:?}", msg);
                }
            }
        });

        Ok(WebRtcClient {
            _connection: peer,
            channel: SendRecvCallbackChannel::new(Box::new(data)).await.unwrap()
        })
    }
}

pub struct WebRtcClient {
    _connection: RtcPeerConnection,
    channel: SendRecvCallbackChannel
}

impl WebRtcClient {
    pub fn channel(&mut self) -> &mut SendRecvCallbackChannel {
        &mut self.channel
    }
}

#[derive(Clone)]
pub struct WebRtcServer {
    signaling: SendRecvCallbackChannel,
    pub clients: Rc<RefCell<HashMap<ConnectionId, SendRecvCallbackChannel>>>,
    pub new_clients: Rc<RefCell<VecDeque<ConnectionId>>>
}

impl WebRtcServer {
    pub async fn new(signaling_url: &str, game_name: &str, server_name: &str) -> Result<WebRtcServer, JsValue> {
        // Register as a server
        let websocket = WebSocket::new(signaling_url)?;
        let mut ws = SendRecvCallbackChannel::new(Box::new(websocket)).await?;
        ws.send(SignalingMessage::Register { game: game_name.to_owned(), name: server_name.to_owned() })?;

        // Discard list of existing servers
        let _: SignalingMessage = ws.recv().await?;

        let server = WebRtcServer {
            signaling: ws.clone(),
            clients: Rc::new(RefCell::new(HashMap::new())),
            new_clients: Rc::new(RefCell::new(VecDeque::new()))
        };

        spawn_local(Self::listen(server.clone()));

        Ok(server)
    }

    async fn listen(server: Self) {
        // Listen for incoming connections
        let signaling_server = server.signaling.clone();
        let mut demux = SignalingDemux::new(signaling_server);
        loop {
            match demux.recv().await {
                Ok(SignalingDemuxRecv::System(msg)) => {
                    console_warn!("ERROR: WebRtcServer.listen(): Unexpected system message: {:?}", msg);
                },
                Ok(SignalingDemuxRecv::Relay(connection_id, client_conn)) => {
                    console_log!("WebRtcServer: Handling new connection...");
                    let server = server.clone();
                    spawn_local(async move {
                        let channel = Self::handle_connection(client_conn).await.unwrap();
                        server.clients.borrow_mut().insert(connection_id.clone(), channel);
                        server.new_clients.borrow_mut().push_back(connection_id.clone());
                        console_log!("WebRtcServer: Added connection {}", connection_id);
                    })
                }
                Err(e) => {
                    console_warn!("ERROR: WebRtcServer.listen(): {:?}. Stopping listening for new connections", e);
                    // TODO if failed, can we reconnect/reregister silently to avoid losing existing clients?
                    return;
                },
            }
        }
    }

    async fn handle_connection(mut client_conn: SignalingClientConnection) -> Result<SendRecvCallbackChannel, JsValue> {
        console_log!("\t\tWebRtcServer.handle_connection(): Started");

        // Receive OFFER from client
        let msg = client_conn.recv().await?;
        let offer_sdp = if let RelayMessage::Offer(offer_sdp) = msg {
            console_log!("\t\tWebRtcServer.handle_connection(): Received OFFER");
            offer_sdp
        } else {
            return Err(JsValue::from_str(&format!("WebRtcServer.handle_connection(): Unexpected msg from signaling server: {:?}", msg)));
        };
        let peer = make_rtc_peer()?;
        let mut offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        offer_obj.sdp(&offer_sdp);
        JsFuture::from(peer.set_remote_description(&offer_obj)).await?;

        // Send ICE candidates to client
        let mut client_sender = client_conn.clone_sender();
        let onicecandidate_callback2 =
            Closure::<dyn FnMut(_)>::new(move |ev: RtcPeerConnectionIceEvent| {
                if let Some(candidate) = ev.candidate() {
                    client_sender.send(RelayMessage::IceCandidate {
                        candidate: candidate.candidate(),
                        sdp_mid: candidate.sdp_mid(),
                        sdp_m_line_index: candidate.sdp_m_line_index()
                    }).unwrap();
                }
            });
        peer.set_onicecandidate(Some(onicecandidate_callback2.as_ref().unchecked_ref()));
        onicecandidate_callback2.forget();

        // Send ANSWER to peer_1
        let answer = JsFuture::from(peer.create_answer()).await?;
        let answer_sdp = Reflect::get(&answer, &JsValue::from_str("sdp"))?.as_string().unwrap();

        let mut answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        answer_obj.sdp(&answer_sdp);
        let sld_promise = peer.set_local_description(&answer_obj);
        JsFuture::from(sld_promise).await?;

        client_conn.send(RelayMessage::Answer(answer_sdp))?;

        // Recv ICE candidates from client
        // Takes ownership of client conn
        let peer_clone = peer.clone();
        spawn_local(async move {
            loop {
                let msg = client_conn.recv().await.unwrap();
                if let RelayMessage::IceCandidate { candidate, sdp_mid, sdp_m_line_index } = msg {
                    let mut init = RtcIceCandidateInit::new(&candidate);
                    let sdp_mid = match &sdp_mid {
                        Some(str) => Some(str.as_str()),
                        None => None
                    };
                    init.sdp_mid(sdp_mid);
                    init.sdp_m_line_index(sdp_m_line_index);
                    let cand = RtcIceCandidate::new(&init).unwrap();
                    JsFuture::from(peer_clone.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&cand))).await.unwrap();
                } else {
                    console_warn!("WebRtcServer.handle_connection(): Recv unexpected while listening for ICE candidates: {:?}", msg);
                }
            }
        });

        // Get data channel
        let peer_clone = peer.clone();
        let data_channel = Rc::new(RefCell::new(None));
        JsFuture::from(Promise::new(&mut |resolve, _| {
            let data_channel_cloned = data_channel.clone();
            let ondatachannel_callback = Closure::<dyn FnMut(_)>::new(move |ev: RtcDataChannelEvent| {
                let resolve = resolve.clone();
                let data_channel = data_channel_cloned.clone();
                spawn_local(async move {
                    *data_channel.borrow_mut() = Some(SendRecvCallbackChannel::new(Box::new(ev.channel())).await.unwrap());
                    resolve.call0(&JsValue::UNDEFINED).unwrap();
                });
            });
            peer_clone.set_ondatachannel(Some(ondatachannel_callback.as_ref().unchecked_ref()));
            ondatachannel_callback.forget();
        })).await?;
        let data_channel = data_channel.borrow_mut().take().expect("WebRtcServer.handle_connection(): Expected data channel to be ready");

        Ok(data_channel)
    }
}
