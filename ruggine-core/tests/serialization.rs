use ruggine_core::*;
use serde_json::{self as json, Value};

fn parse(json_str: &str) -> Value {
    json::from_str(json_str).expect("valid json")
}

/*
    Obiettivo test: Verificare che un WsMessage::SendMessage venga serializzato nel JSON atteso:
    ossia che abbia type "sendMessage" e il payload corretto con campi in camelCase.
    Verificare anche che lo stesso JSON sia deserializzabile di nuovo nello stesso valore Rust SendMessage
*/
#[test]
fn ws_send_message_roundtrip() {
    /* i campi sono snake_case in Rust ma grazie agli attributi serde verranno convertiti in camelCase durante la serializzazione */
    let sm = SendMessage {
        client_msg_id: "11111111-1111-4111-8111-111111111111".to_string(),
        group_id: "22222222-2222-4222-8222-222222222222".to_string(),
        content: "ciao".to_string(),
        sent_at: Some("2025-11-02T10:20:30Z".to_string()),
    };
    let msg = WsMessage::SendMessage(sm.clone());
    // serializzazione in una stringa json
    let s = json::to_string(&msg).expect("serialize");
    // Value è  un albero json generico che ti permette di "navigare" nel json senza dover definire i campi
    let v = parse(&s);

    assert_eq!(v["type"], "sendMessage");
    assert_eq!(v["payload"]["clientMsgId"], sm.client_msg_id);
    assert_eq!(v["payload"]["groupId"], sm.group_id);
    assert_eq!(v["payload"]["content"], sm.content);
    assert_eq!(v["payload"]["sentAt"], sm.sent_at.clone().unwrap());
    // qui la stringa json viene deserializzata, la funzione vede nella stringa json "type" e "payload" e da lì capisce
    // di dover costruire un SendMessage
    let back: WsMessage = json::from_str(&s).expect("deserialize");
    match back {
        /*Qui estrai il payload SendMessage in una nuova variabile locale sm_back  e lo confronti con l’originale sm. */
        WsMessage::SendMessage(sm_back) => assert_eq!(sm_back, sm),
        _ => panic!("expected SendMessage"),
    }
}
/*
    Questo test è simile al precedente ma verifica il caso in cui la data di creazione è None anzicché some(value)
    Obiettivo: verificare che il campo sentAt diventa nullo dopo serializzazione.
*/
#[test]
fn ws_send_message_omits_optional_sent_at() {
    let sm = SendMessage {
        client_msg_id: "11111111-1111-4111-8111-111111111111".to_string(),
        group_id: "22222222-2222-4222-8222-222222222222".to_string(),
        content: "ciao".to_string(),
        sent_at: None,
    };
    let msg = WsMessage::SendMessage(sm.clone());

    let s = json::to_string(&msg).expect("serialize");
    let v = parse(&s);

    assert!(v["payload"]["sentAt"].is_null(), "sentAt should be omitted and thus null in Value access");

    let back: WsMessage = json::from_str(&s).expect("deserialize");
    match back {
        WsMessage::SendMessage(sm_back) => assert_eq!(sm_back, sm),
        _ => panic!("expected SendMessage"),
    }
}

/*
    Obiettivo test: Verificare che un WsMessage::Message venga serializzato nel JSON atteso:
    ossia che abbia type "message" e il payload corretto con campi in camelCase.
    Verificare anche che lo stesso JSON sia deserializzabile di nuovo nello stesso valore Rust Message
*/
#[test]
fn ws_message_roundtrip() {
    let m = Message {
        message_id: "33333333-3333-4333-8333-333333333333".to_string(),
        group_id: "22222222-2222-4222-8222-222222222222".to_string(),
        sender_id: "44444444-4444-4444-8444-444444444444".to_string(),
        content: "hello".to_string(),
        created_at: "2025-11-02T10:20:35Z".to_string(),
    };
    let msg = WsMessage::Message(m.clone());

    let s = json::to_string(&msg).expect("serialize");
    let v = parse(&s);

    assert_eq!(v["type"], "message");
    assert_eq!(v["payload"]["messageId"], m.message_id);
    assert_eq!(v["payload"]["createdAt"], m.created_at);

    let back: WsMessage = json::from_str(&s).expect("deserialize");
    match back {
        WsMessage::Message(m_back) => assert_eq!(m_back, m),
        _ => panic!("expected Message"),
    }
}

/*
    Obiettivo test: Verificare che un WsMessage::Ack con status "ok" venga serializzato nel JSON atteso:
    ossia che abbia type "ack" e il payload corretto con campi in camelCase.
    Verificare anche che lo stesso JSON sia deserializzabile di nuovo nello stesso valore Rust Ack
*/
#[test]
fn ws_ack_ok_roundtrip() {
    let ack = Ack {
        in_reply_to: "11111111-1111-4111-8111-111111111111".to_string(),
        status: AckStatus::Ok,
        message_id: Some("33333333-3333-4333-8333-333333333333".to_string()),
        created_at: Some("2025-11-02T10:20:35Z".to_string()),
        group_id: Some("22222222-2222-4222-8222-222222222222".to_string()),
        content: Some("hello".to_string()),
        error: None,
    };
    let msg = WsMessage::Ack(ack.clone());

    let s = json::to_string(&msg).expect("serialize");
    let v = parse(&s);

    assert_eq!(v["type"], "ack");
    assert_eq!(v["payload"]["status"], "ok");
    assert!(v["payload"]["error"].is_null());

    let back: WsMessage = json::from_str(&s).expect("deserialize");
    match back {
        WsMessage::Ack(ack_back) => assert_eq!(ack_back, ack),
        _ => panic!("expected Ack"),
    }
}
/*
    Obiettivo test: Verificare che un WsMessage::Ack con status "Error" venga serializzato nel JSON atteso:
    ossia che abbia status "error" e il payload corretto con campi in camelCase.
    Verificare anche che lo stesso JSON sia deserializzabile di nuovo nello stesso valore Rust Ack
*/
#[test]
fn ws_ack_error_roundtrip() {
    let err = Error {
        code: "forbidden".to_string(),
        message: "not a member".to_string(),
        details: None,
    };
    let ack = Ack {
        in_reply_to: "11111111-1111-4111-8111-111111111111".to_string(),
        status: AckStatus::Error,
        message_id: None,
        created_at: None,
        group_id: None,
        content: None,
        error: Some(err.clone()),
    };
    let msg = WsMessage::Ack(ack.clone());

    let s = json::to_string(&msg).expect("serialize");
    let v = parse(&s);

    assert_eq!(v["payload"]["status"], "error");
    assert_eq!(v["payload"]["error"]["code"], err.code);

    let back: WsMessage = json::from_str(&s).expect("deserialize");
    match back {
        WsMessage::Ack(ack_back) => assert_eq!(ack_back, ack),
        _ => panic!("expected Ack"),
    }
}

/*
    Obiettivo test: 
    verificare che RegisterResponse venga serializzato nel JSON con i nomi campo giusti (camelCase)
    verificare che lo stesso JSON sia deserializzabile di nuovo nello stesso valore Rust
*/
#[test]
fn http_register_response_roundtrip() {
    let user = User {
        user_id: "55555555-5555-4555-8555-555555555555".to_string(),
        username: "alice".to_string(),
        created_at: "2025-11-02T10:10:10Z".to_string(),
    };
    let resp = RegisterResponse { user: user.clone(), token: "token123".to_string() };

    let s = json::to_string(&resp).expect("serialize");
    let v = parse(&s);

    assert_eq!(v["user"]["userId"], user.user_id);
    assert_eq!(v["user"]["username"], user.username);
    assert_eq!(v["user"]["createdAt"], user.created_at);

    let back: RegisterResponse = json::from_str(&s).expect("deserialize");
    assert_eq!(back.user, user);
    assert_eq!(back.token, "token123");
}

/*
    Obiettivo test: 
    verificare che CreateGroupResponse venga serializzato nel JSON con i nomi campo giusti (camelCase)
    verificare che lo stesso JSON sia deserializzabile di nuovo nello stesso valore Rust
*/
#[test]
fn http_create_group_response_roundtrip() {
    let group = Group {
        group_id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".to_string(),
        name: "general".to_string(),
        created_at: "2025-11-02T10:00:00Z".to_string(),
    };
    let resp = CreateGroupResponse { group: group.clone() };

    let s = json::to_string(&resp).expect("serialize");
    let v = parse(&s);

    assert_eq!(v["group"]["groupId"], group.group_id);
    assert_eq!(v["group"]["name"], group.name);
    assert_eq!(v["group"]["createdAt"], group.created_at);

    let back: CreateGroupResponse = json::from_str(&s).expect("deserialize");
    assert_eq!(back.group, group);
}

/*
    Obiettivo test: 
    verificare che ListMessageResponse venga serializzato nel JSON con i nomi campo giusti (camelCase)
    e che contenga ciascun messaggio.
    verificare che lo stesso JSON sia deserializzabile di nuovo nello stesso valore Rust mantenendo i messaggi che erano in lista
*/
#[test]
fn http_list_messages_response_roundtrip() {
    let m1 = Message {
        message_id: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb".to_string(),
        group_id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".to_string(),
        sender_id: "cccccccc-cccc-4ccc-8ccc-cccccccccccc".to_string(),
        content: "hi".to_string(),
        created_at: "2025-11-02T10:01:00Z".to_string(),
    };
    let m2 = Message {
        message_id: "dddddddd-dddd-4ddd-8ddd-dddddddddddd".to_string(),
        group_id: m1.group_id.clone(),
        sender_id: "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee".to_string(),
        content: "there".to_string(),
        created_at: "2025-11-02T10:02:00Z".to_string(),
    };
    let resp = ListMessagesResponse { messages: vec![m1.clone(), m2.clone()] };

    let s = json::to_string(&resp).expect("serialize");
    let v = parse(&s);

    assert_eq!(v["messages"][0]["messageId"], m1.message_id);
    assert_eq!(v["messages"][1]["messageId"], m2.message_id);

    let back: ListMessagesResponse = json::from_str(&s).expect("deserialize");
    assert_eq!(back.messages, vec![m1, m2]);
}

/*
    Obiettivo test: 
    verificare che Error venga serializzato nel JSON con i nomi campo giusti (camelCase)
    verificare che lo stesso JSON sia deserializzabile di nuovo nello stesso valore Rust
*/
#[test]
fn ws_error_envelope_roundtrip() {
    let err = Error {
        code: "unauthorized".to_string(),
        message: "token expired".to_string(),
        details: Some(json::json!({"reason": "expired", "at": "2025-11-02T11:00:00Z"})),
    };
    let msg = WsMessage::Error(err.clone());

    let s = json::to_string(&msg).expect("serialize");
    let v = parse(&s);

    assert_eq!(v["type"], "error");
    assert_eq!(v["payload"]["code"], err.code);
    assert_eq!(v["payload"]["message"], err.message);
    assert_eq!(v["payload"]["details"]["reason"], "expired");

    let back: WsMessage = json::from_str(&s).expect("deserialize");
    match back {
        WsMessage::Error(err_back) => assert_eq!(err_back, err),
        _ => panic!("expected Error envelope"),
    }
}
