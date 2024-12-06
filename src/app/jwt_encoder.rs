use jsonwebtoken::{errors::Error, Algorithm, EncodingKey, Header};

use super::{
  jwt_decoder::Payload,
  models::{BlockState, ScrollableTxt},
  utils::{get_secret_from_file_or_input, JWTError, JWTResult, SecretType},
  ActiveBlock, App, Route, RouteId, TextAreaInput, TextInput,
};

#[derive(Default)]
pub struct Encoder<'a> {
  pub encoded: ScrollableTxt,
  pub header: TextAreaInput<'a>,
  pub payload: TextAreaInput<'a>,
  pub secret: TextInput,
  pub signature_verified: bool,
  pub blocks: BlockState,
}

impl Encoder<'_> {
  pub fn new(secret: String) -> Self {
    let header = TextAreaInput::new(vec![
      "{".to_string(),
      r#"  "alg": "HS256","#.to_string(),
      r#"  "typ": "JWT""#.to_string(),
      "}".to_string(),
    ]);

    Self {
      header,
      secret: TextInput::new(secret),
      blocks: BlockState::new(vec![
        Route {
          id: RouteId::Encoder,
          active_block: ActiveBlock::EncoderHeader,
        },
        Route {
          id: RouteId::Encoder,
          active_block: ActiveBlock::EncoderPayload,
        },
        Route {
          id: RouteId::Encoder,
          active_block: ActiveBlock::EncoderSecret,
        },
        Route {
          id: RouteId::Encoder,
          active_block: ActiveBlock::EncoderToken,
        },
      ]),
      ..Encoder::default()
    }
  }
}

#[derive(Debug)]
struct EncodeArgs {
  pub header: String,
  /// claims
  pub payload: String,
  /// The secret to sign the JWT with.
  pub secret: String,
}

pub fn encode_jwt_token(app: &mut App) {
  let out = encode_token(&EncodeArgs {
    header: app.data.encoder.header.input.lines().join("\n"),
    payload: app.data.encoder.payload.input.lines().join("\n"),
    secret: app.data.encoder.secret.input.value().to_string(),
  });

  match out {
    Ok(token) => {
      if token != app.data.encoder.encoded.get_txt() {
        app.data.encoder.encoded = ScrollableTxt::new(token);
        app.data.encoder.signature_verified = true;
      }
      app.data.error = String::new();
    }
    Err(e) => {
      app.handle_error(e);
    }
  }
}

fn encode_token(args: &EncodeArgs) -> JWTResult<String> {
  if args.header.is_empty() {
    return Err(String::from("Header should not be empty").into());
  }
  if args.payload.is_empty() {
    return Err(String::from("Payload should not be empty").into());
  }
  let header: Result<Header, serde_json::Error> = serde_json::from_str(&args.header);
  match header {
    Ok(header) => {
      let alg = header.alg;

      let payload: Result<Payload, serde_json::Error> = serde_json::from_str(&args.payload);
      match payload {
        Ok(payload) => {
          let encoding_key = encoding_key_from_secret(&alg, &args.secret)?;
          Ok(jsonwebtoken::encode(&header, &payload, &encoding_key)?)
        }
        Err(e) => Err(format!("Error parsing payload: {:}", e).into()),
      }
    }
    Err(e) => Err(format!("Error parsing header: {:}", e).into()),
  }
}

pub fn encoding_key_from_secret(alg: &Algorithm, secret_string: &str) -> JWTResult<EncodingKey> {
  let (secret, file_type) = get_secret_from_file_or_input(alg, secret_string);
  let secret = secret?;

  match alg {
    Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => match file_type {
      SecretType::Plain => Ok(EncodingKey::from_secret(&secret)),
      SecretType::B64 => {
        EncodingKey::from_base64_secret(std::str::from_utf8(&secret)?).map_err(Error::into)
      }
      _ => Err(JWTError::Internal(format!(
        "Invalid secret file type for {alg:?}"
      ))),
    },
    Algorithm::RS256
    | Algorithm::RS384
    | Algorithm::RS512
    | Algorithm::PS256
    | Algorithm::PS384
    | Algorithm::PS512 => match file_type {
      SecretType::Pem => EncodingKey::from_rsa_pem(&secret).map_err(Error::into),
      SecretType::Der => Ok(EncodingKey::from_rsa_der(&secret)),
      _ => Err(JWTError::Internal(format!(
        "Invalid secret file type for {alg:?}"
      ))),
    },
    Algorithm::ES256 | Algorithm::ES384 => match file_type {
      SecretType::Pem => EncodingKey::from_ec_pem(&secret).map_err(Error::into),
      SecretType::Der => Ok(EncodingKey::from_ec_der(&secret)),
      _ => Err(JWTError::Internal(format!(
        "Invalid secret file type for {alg:?}"
      ))),
    },
    Algorithm::EdDSA => match file_type {
      SecretType::Pem => EncodingKey::from_ed_pem(&secret).map_err(Error::into),
      SecretType::Der => Ok(EncodingKey::from_ed_der(&secret)),
      _ => Err(JWTError::Internal(format!(
        "Invalid secret file type for {alg:?}"
      ))),
    },
  }
}

#[cfg(test)]
mod tests {
  use tui_textarea::TextArea;

  use super::*;
  use crate::app::jwt_decoder::{decode_token, DecodeArgs};

  #[test]
  fn test_encode_hmac_jwt_token_with_valid_payload_and_defaults() {
    let mut app = App::new(None, "secrets".into());

    app.data.encoder.payload.input = vec![
      "{",
      r#"  "sub": "1234567890","#,
      r#"  "name": "John Doe","#,
      r#"  "iat": 1516239022"#,
      "}",
    ]
    .into();

    encode_jwt_token(&mut app);

    assert_eq!(app
      .data
      .encoder
      .encoded
      .get_txt(), "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpYXQiOjE1MTYyMzkwMjIsIm5hbWUiOiJKb2huIERvZSIsInN1YiI6IjEyMzQ1Njc4OTAifQ.TggX4VlPVD-2G5eUT5AhzepyMCx_nuzfiQ_YkdXsMKI");
    assert!(app.data.encoder.signature_verified);

    let args = DecodeArgs {
      jwt: app.data.encoder.encoded.get_txt(),
      secret: String::from("secrets"),
      time_format_utc: false,
      ignore_exp: true,
    };

    let decoded = decode_token(&args).1;

    assert!(decoded.is_ok())
  }

  #[test]
  fn test_encode_rsa_jwt_token_with_valid_payload_and_header() {
    let mut app = App::new(None, "".into());

    let header = vec!["{", r#"  "alg": "RS256","#, r#"  "typ": "JWT""#, "}"];
    app.data.encoder.header.input = header.clone().into();

    let claims = vec![
      "{",
      r#"  "sub": "1234567890","#,
      r#"  "name": "John Doe","#,
      r#"  "iat": 1516239022"#,
      "}",
    ];
    app.data.encoder.payload.input = claims.clone().into();

    app.data.encoder.secret.input = "@./test_data/test_rsa_private_key.pem".into();

    encode_jwt_token(&mut app);
    assert_eq!(app.data.error, "");
    assert_eq!(app
      .data
      .encoder
      .encoded
      .get_txt(), "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJpYXQiOjE1MTYyMzkwMjIsIm5hbWUiOiJKb2huIERvZSIsInN1YiI6IjEyMzQ1Njc4OTAifQ.a6yeSQkIfGD1Va9TgdImZUZ1AKO0OgP15ZFV4JPpZy8TpeByQpqUA3r2kJHNeUlETyEeYMKsDbZI5dYOEa_ZfF9xY6eslV1xmawOPkJYzf8IK3Lb42GEykn9qBWSvHzh5xFs2U1dYjJ9GW7bqhyPVaRVRKh1EBw8AbXmEYT42xSDnzkVUHhPpGM8_2anJNXvnexCQKlVRVVzZC04eHNsRIl5_n50irg7bQCO4z24kwViMTuCQTalV9LXCfdxp7_3Pp4Av_iJtkKHDXWs9GrrD6ttq1J6jOXDSbxn42XrPlxirr0pNtdvbk58W2LqYz4_G9q0HTRz_WO3FmaSxIxyqQ");
    assert!(app.data.encoder.signature_verified);

    // decode the key and verify
    let args = DecodeArgs {
      jwt: app.data.encoder.encoded.get_txt(),
      secret: String::from("@./test_data/test_rsa_public_key.pem"),
      time_format_utc: false,
      ignore_exp: true,
    };

    let decoded = decode_token(&args).1.unwrap();

    assert_eq!(
      decoded.header,
      serde_json::from_str(header.join("\n").as_str()).unwrap()
    );
    assert_eq!(
      decoded.claims,
      serde_json::from_str(claims.join("\n").as_str()).unwrap()
    );
  }

  #[test]
  fn test_encode_rsa_pss_jwt_token_with_valid_payload_and_header() {
    let mut app = App::new(None, "".into());

    let header = vec!["{", r#"  "alg": "PS256","#, r#"  "typ": "JWT""#, "}"];
    app.data.encoder.header.input = header.clone().into();

    let claims = vec![
      "{",
      r#"  "sub": "1234567890","#,
      r#"  "name": "John Doe","#,
      r#"  "iat": 1516239022"#,
      "}",
    ];
    app.data.encoder.payload.input = claims.clone().into();

    app.data.encoder.secret.input = "@./test_data/test_rsa_private_key.der".into();

    encode_jwt_token(&mut app);
    assert_eq!(app.data.error, "");
    assert!(!app.data.encoder.encoded.get_txt().is_empty());
    assert!(app.data.encoder.signature_verified);

    // decode the key and verify
    let args = DecodeArgs {
      jwt: app.data.encoder.encoded.get_txt(),
      secret: String::from("@./test_data/test_rsa_public_key.der"),
      time_format_utc: false,
      ignore_exp: true,
    };

    let decoded = decode_token(&args).1.unwrap();

    assert_eq!(
      decoded.header,
      serde_json::from_str(header.join("\n").as_str()).unwrap()
    );
    assert_eq!(
      decoded.claims,
      serde_json::from_str(claims.join("\n").as_str()).unwrap()
    );
  }

  #[test]
  fn test_encode_ecdsa_jwt_token_with_valid_payload_and_header() {
    let mut app = App::new(None, "".into());

    let header = vec!["{", r#"  "alg": "ES256","#, r#"  "typ": "JWT""#, "}"];
    app.data.encoder.header.input = header.clone().into();

    let claims = vec![
      "{",
      r#"  "sub": "1234567890","#,
      r#"  "name": "John Doe","#,
      r#"  "iat": 1516239022"#,
      "}",
    ];
    app.data.encoder.payload.input = claims.clone().into();

    app.data.encoder.secret.input = "@./test_data/test_ecdsa_private_key.pk8".into();

    encode_jwt_token(&mut app);
    assert_eq!(app.data.error, "");
    assert!(!app.data.encoder.encoded.get_txt().is_empty());
    assert!(app.data.encoder.signature_verified);

    // decode the key and verify
    let args = DecodeArgs {
      jwt: app.data.encoder.encoded.get_txt(),
      secret: String::from("@./test_data/test_ecdsa_public_key.pk8"),
      time_format_utc: false,
      ignore_exp: true,
    };

    let decoded = decode_token(&args).1.unwrap();

    assert_eq!(
      decoded.header,
      serde_json::from_str(header.join("\n").as_str()).unwrap()
    );
    assert_eq!(
      decoded.claims,
      serde_json::from_str(claims.join("\n").as_str()).unwrap()
    );
  }

  #[test]
  fn test_encode_eddsa_jwt_token_with_valid_payload_and_header() {
    let mut app = App::new(None, "".into());

    let header = vec!["{", r#"  "alg": "EdDSA","#, r#"  "typ": "JWT""#, "}"];
    app.data.encoder.header.input = header.clone().into();

    let claims = vec![
      "{",
      r#"  "sub": "1234567890","#,
      r#"  "name": "John Doe","#,
      r#"  "iat": 1516239022"#,
      "}",
    ];
    app.data.encoder.payload.input = claims.clone().into();

    app.data.encoder.secret.input = "@./test_data/test_eddsa_private_key.pem".into();

    encode_jwt_token(&mut app);
    assert_eq!(app.data.error, "");
    assert!(!app.data.encoder.encoded.get_txt().is_empty());
    assert!(app.data.encoder.signature_verified);

    // decode the key and verify
    let args = DecodeArgs {
      jwt: app.data.encoder.encoded.get_txt(),
      secret: String::from("@./test_data/test_eddsa_public_key.pem"),
      time_format_utc: false,
      ignore_exp: true,
    };

    let decoded = decode_token(&args).1.unwrap();

    assert_eq!(
      decoded.header,
      serde_json::from_str(header.join("\n").as_str()).unwrap()
    );
    assert_eq!(
      decoded.claims,
      serde_json::from_str(claims.join("\n").as_str()).unwrap()
    );
  }

  #[test]
  fn test_encode_jwt_token_with_empty_header() {
    let mut app = App::new(None, "".into());

    app.data.encoder.header.input = TextArea::default();

    encode_jwt_token(&mut app);

    assert_eq!(app.data.error, "Header should not be empty");
  }

  #[test]
  fn test_encode_jwt_token_with_empty_payload() {
    let mut app = App::new(None, "".into());

    app.data.encoder.payload.input = TextArea::default();

    encode_jwt_token(&mut app);

    assert_eq!(app.data.error, "Payload should not be empty");
  }

  #[test]
  fn test_encode_jwt_token_with_invalid_header() {
    let mut app = App::new(None, "".into());

    app.data.encoder.header.input = vec!["{", r#"  "sub": "1234567890""#, "}"].into();

    app.data.encoder.payload.input = vec!["{", r#"  "sub": "1234567890""#, "}"].into();

    encode_jwt_token(&mut app);

    assert_eq!(
      app.data.error,
      "Error parsing header: missing field `alg` at line 3 column 1"
    );
  }
}
