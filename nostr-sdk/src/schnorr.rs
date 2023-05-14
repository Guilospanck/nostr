#![allow(dead_code)]

use bitcoin_hashes::{sha256, Hash};
use secp256k1::{
  ecdsa, schnorr, Error, KeyPair, Message, PublicKey, Secp256k1, SecretKey, Signing, Verification,
  XOnlyPublicKey,
};

pub struct AsymmetricKeys {
  pub private_key: SecretKey,
  pub public_key: PublicKey,
}

///
/// Signs an ECDSA signature for a determined content.
///
/// If the process of signing happens correctly, returns the `Signature` created.
/// Otherwise, returns an `Error` with an error message.
///
fn sign_ecdsa<C: Signing>(
  secp: &Secp256k1<C>,
  msg: &[u8],
  seckey: [u8; 32],
) -> Result<ecdsa::Signature, Error> {
  let msg = sha256::Hash::hash(msg);
  let msg = Message::from_slice(&msg)?;
  let seckey = SecretKey::from_slice(&seckey).expect("32 bytes, within curve order");
  Ok(secp.sign_ecdsa(&msg, &seckey))
}

///
/// Verifies an ECDSA signature for a determined content.
///
/// If the signature is verified correctly, returns an `Ok(true)`.
/// Otherwise, `panics` with an error message.
///
fn verify_ecdsa<C: Verification>(
  secp: &Secp256k1<C>,
  msg: &[u8],
  sig: [u8; 64],
  pubkey: [u8; 33],
) -> Result<bool, Error> {
  let msg = sha256::Hash::hash(msg);
  let msg = Message::from_slice(&msg)?;
  let sig = ecdsa::Signature::from_compact(&sig)?;
  let pubkey = PublicKey::from_slice(&pubkey)?;

  match secp.verify_ecdsa(&msg, &sig, &pubkey) {
    Ok(_) => Ok(true),
    Err(err) => panic!("{}", err),
  }
}

///
/// Signs a Schnorr signature for a determined content.
///
/// If the process of signing happens correctly, returns the `Signature` created.
/// Otherwise, returns an `Error` with an error message.
///
fn sign_schnorr<C: Signing>(
  secp: &Secp256k1<C>,
  msg: &[u8],
  seckey: [u8; 32],
) -> Result<schnorr::Signature, Error> {
  let msg = sha256::Hash::hash(msg);
  let msg = Message::from_slice(&msg)?;
  let seckey = SecretKey::from_slice(&seckey).expect("32 bytes, within curve order");
  let keypair = KeyPair::from_secret_key(secp, &seckey);
  Ok(secp.sign_schnorr_no_aux_rand(&msg, &keypair))
}

///
/// Verifies a Schnorr signature for a determined content.
///
/// If the signature is verified correctly, returns an `Ok(true)`.
/// Otherwise, `panics` with an error message.
///
fn verify_schnorr<C: Verification>(
  secp: &Secp256k1<C>,
  msg: &[u8],
  sig: schnorr::Signature,
  keypair: KeyPair,
) -> Result<bool, Error> {
  let msg = sha256::Hash::hash(msg);
  let msg = Message::from_slice(&msg)?;
  let pubkey = XOnlyPublicKey::from_keypair(&keypair);

  match secp.verify_schnorr(&sig, &msg, &pubkey.0) {
    Ok(_) => Ok(true),
    Err(err) => panic!("{}", err),
  }
}

///
/// Generates random keypairs (private and public keys) that
/// can be used for both Schnorr and ECDSA signatures.
///
pub fn generate_keys() -> AsymmetricKeys {
  let secp = Secp256k1::new();
  let mut rng = rand::thread_rng();

  let (seckey, pubkey) = secp.generate_keypair(&mut rng);
  assert_eq!(pubkey, PublicKey::from_secret_key(&secp, &seckey));

  AsymmetricKeys {
    public_key: pubkey,
    private_key: seckey,
  }
}

#[cfg(test)]
mod tests {
  use bitcoin_hashes::hex::ToHex;
  use secp256k1::All;

  use super::*;

  struct Sut {
    seckey: [u8; 32],
    pubkey: [u8; 33],
    msg: Vec<u8>,
    secp: Secp256k1<All>,
  }

  fn make_sut() -> Sut {
    let seckey = [
      59, 148, 11, 85, 134, 130, 61, 253, 2, 174, 59, 70, 27, 180, 51, 107, 94, 203, 174, 253, 102,
      39, 170, 146, 46, 252, 4, 143, 236, 12, 136, 28,
    ];
    let pubkey = [
      2, 29, 21, 35, 7, 198, 183, 43, 14, 208, 65, 139, 14, 112, 205, 128, 231, 245, 41, 91, 141,
      134, 245, 114, 45, 63, 82, 19, 251, 210, 57, 79, 54,
    ];
    let msg = b"This is some message";

    let secp = Secp256k1::new();

    Sut {
      seckey,
      pubkey,
      msg: msg.to_vec(),
      secp,
    }
  }

  #[test]
  fn test_should_sign_schnorr_without_errors() {
    let sut: Sut = make_sut();
    assert!(sign_schnorr(&sut.secp, &sut.msg, sut.seckey).is_ok());
  }

  #[test]
  #[should_panic(expected = "32 bytes, within curve order: InvalidSecretKey")]
  fn test_should_return_an_error_when_trying_to_sign_schnorr_with_invalid_secret_key() {
    let sut: Sut = make_sut();
    let invalid_seckey = [0x00; 32];
    sign_schnorr(&sut.secp, &sut.msg, invalid_seckey).unwrap();
  }

  #[test]
  fn test_should_verify_schnorr_without_errors() {
    let sut: Sut = make_sut();
    let signature_schnorr = sign_schnorr(&sut.secp, &sut.msg, sut.seckey).unwrap();
    let seckey = SecretKey::from_slice(&sut.seckey).unwrap();
    let keypair = KeyPair::from_secret_key(&sut.secp, &seckey);

    assert!(verify_schnorr(&sut.secp, &sut.msg, signature_schnorr, keypair).is_ok());
  }

  #[test]
  #[should_panic(expected = "malformed signature")]
  fn test_should_return_err_when_schnorr_signature_is_invalid_for_msg() {
    let sut: Sut = make_sut();
    let invalid_signature_schnorr =
      sign_schnorr(&sut.secp, b"another message", sut.seckey).unwrap();
    let seckey = SecretKey::from_slice(&sut.seckey).unwrap();
    let keypair = KeyPair::from_secret_key(&sut.secp, &seckey);

    verify_schnorr(&sut.secp, &sut.msg, invalid_signature_schnorr, keypair).unwrap();
  }

  #[test]
  fn test_should_sign_ecdsa_without_errors() {
    let sut: Sut = make_sut();
    assert!(sign_ecdsa(&sut.secp, &sut.msg, sut.seckey).is_ok());
  }

  #[test]
  #[should_panic(expected = "32 bytes, within curve order: InvalidSecretKey")]
  fn test_should_return_an_error_when_trying_to_sign_ecdsa_with_invalid_secret_key() {
    let sut: Sut = make_sut();
    let invalid_seckey = [0x00; 32];
    sign_ecdsa(&sut.secp, &sut.msg, invalid_seckey).unwrap();
  }

  #[test]
  fn test_should_verify_ecdsa_without_errors() {
    let sut: Sut = make_sut();
    let signature_ecdsa = sign_ecdsa(&sut.secp, &sut.msg, sut.seckey)
      .unwrap()
      .serialize_compact();

    assert!(verify_ecdsa(&sut.secp, &sut.msg, signature_ecdsa, sut.pubkey).is_ok());
  }

  #[test]
  #[should_panic(expected = "signature failed verification")]
  fn test_should_return_err_when_ecdsa_signature_is_invalid_for_msg() {
    let sut: Sut = make_sut();
    let invalid_signature_ecdsa = sign_ecdsa(&sut.secp, b"another message", sut.seckey)
      .unwrap()
      .serialize_compact();

    verify_ecdsa(&sut.secp, &sut.msg, invalid_signature_ecdsa, sut.pubkey).unwrap();
  }

  #[test]
  fn test_that_public_key_used_with_ecdsa_can_also_be_used_with_schnorr_if_dropping_first_byte() {
    let sut: Sut = make_sut();

    // ECDSA
    let signature_ecdsa = sign_ecdsa(&sut.secp, &sut.msg, sut.seckey)
      .unwrap()
      .serialize_compact();
    assert!(verify_ecdsa(&sut.secp, &sut.msg, signature_ecdsa, sut.pubkey).is_ok());

    // Schnorr
    let signature_schnorr = sign_schnorr(&sut.secp, &sut.msg, sut.seckey).unwrap();
    let seckey = SecretKey::from_slice(&sut.seckey).unwrap();
    let keypair = KeyPair::from_secret_key(&sut.secp, &seckey);
    assert!(verify_schnorr(&sut.secp, &sut.msg, signature_schnorr, keypair).is_ok());

    // Get Public Key without first byte
    let public_key_without_first_byte = sut.pubkey[1..].to_hex();

    // Get public key used in Schnorr
    let public_key_used_schnorr = XOnlyPublicKey::from_keypair(&keypair).0.to_hex();

    // assert that both public keys are equal
    assert_eq!(public_key_without_first_byte, public_key_used_schnorr)
  }
}
