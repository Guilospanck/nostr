use bitcoin_hashes::{hex::ToHex, sha256, Hash};
use secp256k1::{
  ecdsa, schnorr, Error, KeyPair, Message, PublicKey, Secp256k1, SecretKey, Signing, Verification,
  XOnlyPublicKey,
};

fn verify_edcsa<C: Verification>(
  secp: &Secp256k1<C>,
  msg: &[u8],
  sig: [u8; 64],
  pubkey: [u8; 33],
) -> Result<bool, Error> {
  let msg = sha256::Hash::hash(msg);
  let msg = Message::from_slice(&msg)?;
  let sig = ecdsa::Signature::from_compact(&sig)?;
  let pubkey = PublicKey::from_slice(&pubkey)?;

  Ok(secp.verify_ecdsa(&msg, &sig, &pubkey).is_ok())
}

fn verify_schnorr<C: Verification>(
  secp: &Secp256k1<C>,
  msg: &[u8],
  sig: schnorr::Signature,
  keypair: KeyPair,
) -> Result<bool, Error> {
  let msg = sha256::Hash::hash(msg);
  let msg = Message::from_slice(&msg)?;
  let pubkey = XOnlyPublicKey::from_keypair(&keypair);

  Ok(secp.verify_schnorr(&sig, &msg, &pubkey.0).is_ok())
}

fn sign_ecdsa<C: Signing>(
  secp: &Secp256k1<C>,
  msg: &[u8],
  seckey: [u8; 32],
) -> Result<ecdsa::Signature, Error> {
  let msg = sha256::Hash::hash(msg);
  let msg = Message::from_slice(&msg)?;
  let seckey = SecretKey::from_slice(&seckey)?;
  Ok(secp.sign_ecdsa(&msg, &seckey))
}

fn sign_schnorr<C: Signing>(
  secp: &Secp256k1<C>,
  msg: &[u8],
  seckey: [u8; 32],
) -> Result<schnorr::Signature, Error> {
  let msg = sha256::Hash::hash(msg);
  let msg = Message::from_slice(&msg)?;
  let seckey = SecretKey::from_slice(&seckey)?;
  let keypair = KeyPair::from_secret_key(secp, &seckey);
  Ok(secp.sign_schnorr_no_aux_rand(&msg, &keypair))
}

fn generate_keys() {
  let secp = Secp256k1::new();
  let mut rng = rand::thread_rng();

  let (seckey, pubkey) = secp.generate_keypair(&mut rng);

  println!("Secret Key: {:?}\nPublic Key: {}", seckey, pubkey);

  assert_eq!(pubkey, PublicKey::from_secret_key(&secp, &seckey));
}

fn main() {
  generate_keys();
  let secp = Secp256k1::new();

  let seckey = [
    59, 148, 11, 85, 134, 130, 61, 253, 2, 174, 59, 70, 27, 180, 51, 107, 94, 203, 174, 253, 102,
    39, 170, 146, 46, 252, 4, 143, 236, 12, 136, 28,
  ];
  let pubkey = [
    2, 29, 21, 35, 7, 198, 183, 43, 14, 208, 65, 139, 14, 112, 205, 128, 231, 245, 41, 91, 141,
    134, 245, 114, 45, 63, 82, 19, 251, 210, 57, 79, 54,
  ];
  let msg = b"This is some message";

  // ECDSA
  let signature_ecdsa = sign_ecdsa(&secp, msg, seckey).unwrap();
  let serialize_sig_ecdsa = signature_ecdsa.serialize_compact();
  println!("ECDSA => {:?}\n", serialize_sig_ecdsa.to_hex()); // 64 bytes
  assert!(verify_edcsa(&secp, msg, serialize_sig_ecdsa, pubkey).unwrap());

  // Schnorr
  let signature_schnorr = sign_schnorr(&secp, msg, seckey).unwrap();
  println!("Schnorr => {:?}", signature_schnorr.to_hex()); // 64 bytes
  let seckey = SecretKey::from_slice(&seckey).unwrap();
  let keypair = KeyPair::from_secret_key(&secp, &seckey);
  assert!(verify_schnorr(&secp, msg, signature_schnorr, keypair).unwrap());
}
