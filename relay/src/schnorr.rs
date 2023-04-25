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

