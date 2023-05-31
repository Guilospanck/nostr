#![allow(dead_code)]

use std::str::FromStr;

use bitcoin_hashes::{hex::FromHex, sha256};
use secp256k1::{
  ecdsa, schnorr, KeyPair, Message, PublicKey, Secp256k1, SecretKey, Signing, Verification,
  XOnlyPublicKey,
};

#[derive(Debug)]
pub struct AsymmetricKeys {
  pub private_key: SecretKey,
  pub public_key: PublicKey,
}

impl Default for AsymmetricKeys {
  fn default() -> Self {
    let secp = Secp256k1::new();
    let private_key = SecretKey::new(&mut rand::thread_rng());
    Self {
      private_key,
      public_key: PublicKey::from_secret_key(&secp, &private_key)
    }
  }
}

/// [`Schnorr`] error
#[derive(thiserror::Error, Debug)]
pub enum SchnorrError {
  /// Error related to bitcoin_hashes::hex
  #[error(transparent)]
  SHA256(#[from] bitcoin_hashes::hex::Error),

  /// Error secp256k1
  #[error(transparent)]
  SECP256K1(#[from] secp256k1::Error),
}

///
/// Signs an ECDSA signature for a determined content.
///
/// If the process of signing happens correctly, returns the `Signature` created.
/// Otherwise, returns a `SchnorrError` with an error message.
///
/// ## Arguments
///
/// * `secp` - A Secp256k1 engine to execute signature.
/// * `msg` - A SHA256 hashed message.
/// * `seckey` - The Private Key to sign the message.
///
/// ## Examples
///
/// ```
///     use nostr_sdk::schnorr::*;
///     use secp256k1::Secp256k1;
///     use bitcoin_hashes::{sha256, hex::ToHex, Hash};
/// 
///     let seckey = [
///      59, 148, 11, 85, 134, 130, 61, 253, 2, 174, 59, 70, 27, 180, 51, 107, 94, 203, 174, 253, 102,
///      39, 170, 146, 46, 252, 4, 143, 236, 12, 136, 28,
///     ];
///     let hashed_msg = sha256::Hash::hash(b"This is some message");
///     let msg = hashed_msg.to_hex();
///     let secp = Secp256k1::new();
///     assert!(sign_ecdsa(&secp, msg, seckey.to_vec()).is_ok());
/// ```
pub fn sign_ecdsa<C: Signing>(
  secp: &Secp256k1<C>,
  msg: String,
  seckey: Vec<u8>,
) -> Result<ecdsa::Signature, SchnorrError> {
  let hash_from_hex = sha256::Hash::from_hex(&msg)?;
  let msg = Message::from_slice(hash_from_hex.as_ref())?;
  match SecretKey::from_slice(&seckey) {
    Ok(seckey) => Ok(secp.sign_ecdsa(&msg, &seckey)),
    Err(err) => {
      log::error!("[sign_ecdsa] {err}");
      Err(SchnorrError::SECP256K1(err))
    }
  }
}

///
/// Verifies an ECDSA signature for a determined content.
///
/// If the signature is verified correctly, returns an `Ok(true)`.
/// Otherwise, returns a `SchnorrError`.
///
/// ## Arguments
///
/// * `secp` - A Secp256k1 engine to execute verification.
/// * `msg` - A SHA256 hashed message.
/// * `sig` - The ecdsa signature to verify.
/// * `pubkey` - The Public Key to verify against.
///
/// ## Examples
///
/// ```
///     use nostr_sdk::schnorr::*;
///     use std::str::FromStr;
///     use secp256k1::{Secp256k1, ecdsa};
/// 
///     let seckey = [
///      59, 148, 11, 85, 134, 130, 61, 253, 2, 174, 59, 70, 27, 180, 51, 107, 94, 203, 174, 253, 102,
///      39, 170, 146, 46, 252, 4, 143, 236, 12, 136, 28,
///     ];
///     let secp = Secp256k1::new();
///     let sig = match secp256k1::ecdsa::Signature::from_str("bf073c935f71de50ec72bdb79f75b0bf32f9049305c3b22f97c06422c6f2edc86e0d7e07d7d7222678b238b1daee071be5f6fa653c611971395ec0d1c6407caf") {
///       Ok(signature) => signature,
///       Err(_) => return,
///     };
///     let msg = "00960bd35499f8c63a4f65e79d6b1a2b7f1b8c97e76652325567b78c496350ae".to_string(); // already hashed message
///     let pubkey = "614a695bab54e8dc98946abdb8ec019599ece6dada0c23890977d0fa128081d6".to_string();
///     let signature_ecdsa = sign_ecdsa(&secp, msg.clone(), seckey.to_vec())
///      .unwrap();
///      assert!(verify_ecdsa(&secp, msg, signature_ecdsa, pubkey).is_ok());
/// ```
pub fn verify_ecdsa<C: Verification>(
  secp: &Secp256k1<C>,
  msg: String,
  sig: secp256k1::ecdsa::Signature,
  pubkey: String,
) -> Result<bool, SchnorrError> {
  let hash_from_hex = sha256::Hash::from_hex(&msg)?;
  let msg = Message::from_slice(hash_from_hex.as_ref())?;
  let pubkey = PublicKey::from_str(&pubkey)?;

  match secp.verify_ecdsa(&msg, &sig, &pubkey) {
    Ok(_) => Ok(true),
    Err(err) => {
      log::error!("[verify_ecdsa] {err}");
      Err(SchnorrError::SECP256K1(err))
    }
  }
}

///
/// Signs a Schnorr signature for a determined content.
///
/// If the process of signing happens correctly, returns the `Signature` created.
/// Otherwise, returns a `SchnorrError` with an error message.
///
/// ## Arguments
///
/// * `secp` - A Secp256k1 engine to execute signature.
/// * `msg` - A SHA256 hashed message.
/// * `seckey` - The Private Key to sign the message.
///
/// ## Examples
///
/// ```
///     use nostr_sdk::schnorr::*;
///     use secp256k1::Secp256k1;
///     use bitcoin_hashes::{hex::ToHex, sha256, Hash};
/// 
///     let seckey = [
///      59, 148, 11, 85, 134, 130, 61, 253, 2, 174, 59, 70, 27, 180, 51, 107, 94, 203, 174, 253, 102,
///      39, 170, 146, 46, 252, 4, 143, 236, 12, 136, 28,
///     ];
///     let hashed_msg = sha256::Hash::hash(b"This is some message");
///     let msg = hashed_msg.to_hex();
///     let secp = Secp256k1::new();
///     assert!(sign_schnorr(&secp, msg, seckey.to_vec()).is_ok());
/// ```
pub fn sign_schnorr<C: Signing>(
  secp: &Secp256k1<C>,
  msg: String,
  seckey: Vec<u8>,
) -> Result<schnorr::Signature, SchnorrError> {
  let hash_from_hex = sha256::Hash::from_hex(&msg)?;
  let msg = Message::from_slice(hash_from_hex.as_ref())?;
  match SecretKey::from_slice(&seckey) {
    Ok(seckey) => {
      let keypair = KeyPair::from_secret_key(secp, &seckey);
      Ok(secp.sign_schnorr_no_aux_rand(&msg, &keypair))
    }
    Err(err) => {
      log::error!("[sign_schnorr > SecretKey::from_slice] {err}");
      Err(SchnorrError::SECP256K1(err))
    }
  }
}

///
/// Verifies a Schnorr signature for a determined content.
///
/// If the signature is verified correctly, returns an `Ok(true)`.
/// Otherwise, returns a `SchnorrError` with an error message.
///
/// ## Arguments
///
/// * `secp` - A Secp256k1 engine to execute verification.
/// * `msg` - A SHA256 hashed message.
/// * `sig` - The schnorr signature to verify.
/// * `pubkey` - The Public Key to verify against.
///
/// ## Examples
///
/// ```
///     use nostr_sdk::schnorr::*;
///     use std::str::FromStr;
///     use secp256k1::{Secp256k1, schnorr};
/// 
///     let secp = Secp256k1::new();
///     let sig = match schnorr::Signature::from_str("bf073c935f71de50ec72bdb79f75b0bf32f9049305c3b22f97c06422c6f2edc86e0d7e07d7d7222678b238b1daee071be5f6fa653c611971395ec0d1c6407caf") {
///       Ok(signature) => signature,
///       Err(_) => return,
///     };
///     let id = "00960bd35499f8c63a4f65e79d6b1a2b7f1b8c97e76652325567b78c496350ae".to_string(); // already hashed message
///     let pubkey = "614a695bab54e8dc98946abdb8ec019599ece6dada0c23890977d0fa128081d6".to_string();
///     let result = match verify_schnorr(&secp, id.clone(), sig, pubkey.clone()) {
///       Ok(result) => result,
///       Err(_) => return,
///     };
///     assert_eq!(result, true);
/// ```
pub fn verify_schnorr<C: Verification>(
  secp: &Secp256k1<C>,
  msg: String,
  sig: schnorr::Signature,
  pubkey: String,
) -> Result<bool, SchnorrError> {
  let hash_from_hex = sha256::Hash::from_hex(&msg)?;
  let msg = Message::from_slice(hash_from_hex.as_ref())?;
  let x_only_pubkey = XOnlyPublicKey::from_str(&pubkey)?;

  match secp.verify_schnorr(&sig, &msg, &x_only_pubkey) {
    Ok(_) => Ok(true),
    Err(err) => {
      log::error!("[verify_schnorr] {err}");
      Err(SchnorrError::SECP256K1(err))
    }
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
  use std::str::FromStr;

  use bitcoin_hashes::{hex::ToHex, Hash};
  use secp256k1::All;

  use super::*;

  struct Sut {
    seckey: [u8; 32],
    pubkey: [u8; 33],
    msg: String,
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
    let hashed_msg = sha256::Hash::hash(b"This is some message");
    let msg = hashed_msg.to_hex();

    let secp = Secp256k1::new();

    Sut {
      seckey,
      pubkey,
      msg,
      secp,
    }
  }

  #[test]
  fn test_should_sign_schnorr_without_errors() {
    let sut: Sut = make_sut();
    assert!(sign_schnorr(&sut.secp, sut.msg, sut.seckey.to_vec()).is_ok());
  }

  #[test]
  fn test_should_return_an_error_when_trying_to_sign_schnorr_with_invalid_secret_key() {
    let sut: Sut = make_sut();
    let invalid_seckey = [0x00; 32];
    let result = sign_schnorr(&sut.secp, sut.msg, invalid_seckey.to_vec());
    assert!(result.is_err());
    let expected_err_message = String::from("malformed or out-of-range secret key");
    let err_message = result.err().unwrap().to_string();
    assert_eq!(expected_err_message, err_message);
  }

  #[test]
  fn test_should_verify_schnorr_without_errors() {
    let sut: Sut = make_sut();
    let signature_schnorr = sign_schnorr(&sut.secp, sut.msg.clone(), sut.seckey.to_vec()).unwrap();
    let seckey = SecretKey::from_slice(&sut.seckey).unwrap();
    let keypair = KeyPair::from_secret_key(&sut.secp, &seckey);
    let pubkey = XOnlyPublicKey::from_keypair(&keypair);
    assert!(verify_schnorr(&sut.secp, sut.msg, signature_schnorr, pubkey.0.to_string()).is_ok());
  }

  #[test]
  fn verify_schnorr_event_data() {
    let sut: Sut = make_sut();
    let msg = "00960bd35499f8c63a4f65e79d6b1a2b7f1b8c97e76652325567b78c496350ae".to_string();
    let pubkey = "614a695bab54e8dc98946abdb8ec019599ece6dada0c23890977d0fa128081d6".to_string();
    let sig = schnorr::Signature::from_str("bf073c935f71de50ec72bdb79f75b0bf32f9049305c3b22f97c06422c6f2edc86e0d7e07d7d7222678b238b1daee071be5f6fa653c611971395ec0d1c6407caf").unwrap();
    assert!(verify_schnorr(&sut.secp, msg, sig, pubkey).is_ok());
  }

  #[test]
  fn test_should_return_err_when_schnorr_signature_is_invalid_for_msg() {
    let sut: Sut = make_sut();
    let hashed_msg = sha256::Hash::hash(b"another message");
    let msg = hashed_msg.to_hex();
    let invalid_signature_schnorr = sign_schnorr(&sut.secp, msg, sut.seckey.to_vec()).unwrap();
    let seckey = SecretKey::from_slice(&sut.seckey).unwrap();
    let keypair = KeyPair::from_secret_key(&sut.secp, &seckey);
    let pubkey = XOnlyPublicKey::from_keypair(&keypair);
    let result = verify_schnorr(
      &sut.secp,
      sut.msg,
      invalid_signature_schnorr,
      pubkey.0.to_string(),
    );
    assert!(result.is_err());
    let expected_err_message = String::from("malformed signature");
    let err_message = result.err().unwrap().to_string();
    assert_eq!(expected_err_message, err_message);
  }

  #[test]
  fn test_should_sign_ecdsa_without_errors() {
    let sut: Sut = make_sut();
    assert!(sign_ecdsa(&sut.secp, sut.msg, sut.seckey.to_vec()).is_ok());
  }

  #[test]
  fn test_should_return_an_error_when_trying_to_sign_ecdsa_with_invalid_secret_key() {
    let sut: Sut = make_sut();
    let invalid_seckey = [0x00; 32];
    let result = sign_ecdsa(&sut.secp, sut.msg, invalid_seckey.to_vec());
    assert!(result.is_err());
    let expected_err_message = String::from("malformed or out-of-range secret key");
    let err_message = result.err().unwrap().to_string();
    assert_eq!(expected_err_message, err_message);
  }

  #[test]
  fn test_should_verify_ecdsa_without_errors() {
    let sut: Sut = make_sut();
    let signature_ecdsa = sign_ecdsa(&sut.secp, sut.msg.clone(), sut.seckey.to_vec()).unwrap();
    assert!(verify_ecdsa(&sut.secp, sut.msg, signature_ecdsa, sut.pubkey.to_hex()).is_ok());
  }

  #[test]
  fn test_should_return_err_when_ecdsa_signature_is_invalid_for_msg() {
    let sut: Sut = make_sut();
    let hashed_msg = sha256::Hash::hash(b"another message");
    let msg = hashed_msg.to_hex();
    let invalid_signature_ecdsa = sign_ecdsa(&sut.secp, msg, sut.seckey.to_vec()).unwrap();
    let result = verify_ecdsa(
      &sut.secp,
      sut.msg,
      invalid_signature_ecdsa,
      sut.pubkey.to_hex(),
    );
    assert!(result.is_err());
    let expected_err_message = String::from("signature failed verification");
    let err_message = result.err().unwrap().to_string();
    assert_eq!(expected_err_message, err_message);
  }

  #[test]
  fn test_that_public_key_used_with_ecdsa_can_also_be_used_with_schnorr_if_dropping_first_byte() {
    let sut: Sut = make_sut();

    // ECDSA
    let signature_ecdsa = sign_ecdsa(&sut.secp, sut.msg.clone(), sut.seckey.to_vec()).unwrap();
    assert!(verify_ecdsa(
      &sut.secp,
      sut.msg.clone(),
      signature_ecdsa,
      sut.pubkey.to_hex()
    )
    .is_ok());

    // Schnorr
    let signature_schnorr = sign_schnorr(&sut.secp, sut.msg.clone(), sut.seckey.to_vec()).unwrap();
    let seckey = SecretKey::from_slice(&sut.seckey).unwrap();
    let keypair = KeyPair::from_secret_key(&sut.secp, &seckey);
    let pubkey = XOnlyPublicKey::from_keypair(&keypair);
    assert!(verify_schnorr(&sut.secp, sut.msg, signature_schnorr, pubkey.0.to_string()).is_ok());

    // Get Public Key without first byte
    let public_key_without_first_byte = sut.pubkey[1..].to_hex();

    // Get public key used in Schnorr
    let public_key_used_schnorr = XOnlyPublicKey::from_keypair(&keypair).0.to_hex();

    // assert that both public keys are equal
    assert_eq!(public_key_without_first_byte, public_key_used_schnorr)
  }
}
