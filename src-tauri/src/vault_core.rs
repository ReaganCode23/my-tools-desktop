//! Versioned authenticated vault storage. No UI, IPC, network, or plaintext file writes.
use aes_gcm::{aead::{Aead, Payload}, Aes256Gcm, KeyInit, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{io::Write, path::Path, time::{SystemTime, UNIX_EPOCH}};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

pub const MAX_FILE: usize = 9 * 1024 * 1024;
const MAGIC: &[u8;8] = b"MTVAULT1";
const HEADER: usize = 36;
pub type Key = Zeroizing<[u8;32]>;
pub fn now() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() }
fn random<const N:usize>() -> Result<[u8;N],String> { let mut bytes=[0;N];OsRng.try_fill_bytes(&mut bytes).map_err(|_|"System randomness is unavailable".to_string())?;Ok(bytes) }
pub fn new_id() -> Result<String,String> { use base64::Engine;Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random::<16>()?)) }
#[derive(Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct Entry {
 pub id:String,pub title:String,pub username:String,pub password:String,pub url:String,pub notes:String,
 #[serde(default)] pub folder:String,#[serde(default)] pub favourite:bool,#[serde(default)] pub deleted:bool,
 pub created:u64,pub modified:u64,
}
#[derive(Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct Vault {pub version:u32,pub entries:Vec<Entry>}
impl Default for Vault {fn default()->Self {Self{version:1,entries:vec![]}}}
pub fn validate(vault:&Vault)->Result<(),String>{
 if vault.version!=1||vault.entries.len()>2000 {return Err("Unsupported vault or entry limit exceeded".into());}
 let mut seen=std::collections::HashSet::new();
 for e in &vault.entries {if e.id.is_empty()||!seen.insert(&e.id)||e.id.len()>64||e.title.trim().is_empty()||e.title.chars().count()>200||e.username.len()>2048||e.password.len()>4096||e.url.len()>2048||e.notes.len()>32768||e.folder.len()>200{return Err("Invalid entry or field length exceeded".into());}}
 Ok(())
}
pub fn derive(password:&str,salt:&[u8;16])->Result<Key,String>{
 if password.is_empty()||password.len()>4096{return Err("Enter a valid master password".into());}
 let params=Params::new(64*1024,3,1,Some(32)).map_err(|_|"Invalid key derivation parameters")?;
 let argon=Argon2::new(Algorithm::Argon2id,Version::V0x13,params);let mut key=Zeroizing::new([0;32]);
 argon.hash_password_into(password.as_bytes(),salt,&mut key[..]).map_err(|_|"Could not derive encryption key")?;Ok(key)
}
pub fn new_salt()->Result<[u8;16],String>{random()}
pub fn seal(vault:&Vault,key:&Key,salt:&[u8;16])->Result<Vec<u8>,String>{
 validate(vault)?;let plain=Zeroizing::new(serde_json::to_vec(vault).map_err(|_|"Could not serialize vault")?);if plain.len()>MAX_FILE-1024{return Err("Vault size limit exceeded".into());}
 let nonce=random::<12>()?;let mut header=Vec::with_capacity(HEADER);header.extend_from_slice(MAGIC);header.extend_from_slice(salt);header.extend_from_slice(&nonce);
 let cipher=Aes256Gcm::new_from_slice(&key[..]).map_err(|_|"Invalid encryption key")?;
 let encrypted=cipher.encrypt(Nonce::from_slice(&nonce),Payload{msg:&plain,aad:&header}).map_err(|_|"Could not encrypt vault")?;
 header.extend_from_slice(&encrypted);Ok(header)
}
pub fn header_salt(bytes:&[u8])->Result<[u8;16],String>{
 if bytes.len()<HEADER+16||bytes.len()>MAX_FILE||&bytes[..8]!=MAGIC{return Err("This is not a supported MyTools vault".into());}
 bytes[8..24].try_into().map_err(|_|"Invalid vault header".into())
}
pub fn open(bytes:&[u8],key:&Key)->Result<Vault,String>{
 header_salt(bytes)?;let cipher=Aes256Gcm::new_from_slice(&key[..]).map_err(|_|"Invalid encryption key")?;
 let plain=Zeroizing::new(cipher.decrypt(Nonce::from_slice(&bytes[24..36]),Payload{msg:&bytes[36..],aad:&bytes[..36]}).map_err(|_|"Unable to unlock: incorrect password or damaged vault".to_string())?);
 let vault:Vault=serde_json::from_slice(&plain).map_err(|_|"Vault contents are invalid")?;validate(&vault)?;Ok(vault)
}
pub fn fingerprint(bytes:&[u8])->[u8;32]{Sha256::digest(bytes).into()}
pub fn read(path:&Path)->Result<Vec<u8>,String>{
 let meta=std::fs::symlink_metadata(path).map_err(|_|"Could not read the vault file")?;if !meta.is_file()||meta.file_type().is_symlink()||meta.len()>MAX_FILE as u64{return Err("Invalid vault file or size limit exceeded".into());}
 let bytes=std::fs::read(path).map_err(|_|"Could not read the vault file")?;header_salt(&bytes)?;Ok(bytes)
}
fn atomic_write(path:&Path,bytes:&[u8],no_replace:bool)->Result<(),String>{
 let parent=path.parent().ok_or("Invalid vault location")?;std::fs::create_dir_all(parent).map_err(|_|"Could not create vault folder")?;
 let mut staged=tempfile::NamedTempFile::new_in(parent).map_err(|_|"Could not stage encrypted vault")?;
 #[cfg(unix)] {use std::os::unix::fs::PermissionsExt;staged.as_file().set_permissions(std::fs::Permissions::from_mode(0o600)).map_err(|_|"Could not protect vault permissions")?;}
 staged.write_all(bytes).map_err(|_|"Could not write encrypted vault")?;staged.as_file().sync_all().map_err(|_|"Could not synchronize encrypted vault")?;
 if no_replace {staged.persist_noclobber(path).map_err(|_|"A vault already exists; it was not replaced")?;}else{staged.persist(path).map_err(|_|"Could not commit encrypted vault; original is unchanged")?;}
 #[cfg(unix)] {if let Ok(directory)=std::fs::File::open(parent){let _=directory.sync_all();}}
 Ok(())
}
pub fn create(path:&Path,bytes:&[u8])->Result<(),String>{header_salt(bytes)?;atomic_write(path,bytes,true)}
pub fn commit(path:&Path,bytes:&[u8],expected:&[u8;32])->Result<(),String>{
 let previous=read(path)?;if &fingerprint(&previous)!=expected{return Err("Vault changed outside MyTools. Lock and reopen it before saving.".into());}
 // Only encrypted bytes are written, including the previous-version recovery copy.
 atomic_write(&path.with_extension("mtvault.bak"),&previous,false)?;atomic_write(path,bytes,false)
}
pub fn export_encrypted(path:&Path,bytes:&[u8])->Result<(),String>{header_salt(bytes)?;atomic_write(path,bytes,false)}

#[cfg(test)]mod tests{
 use super::*;
 fn sample()->Vault{Vault{version:1,entries:vec![Entry{id:"test-id".into(),title:"Private test account".into(),username:"private@example.invalid".into(),password:"never-write-this-secret".into(),url:"https://example.invalid".into(),notes:"Private note".into(),folder:"Work".into(),favourite:true,deleted:false,created:1,modified:1}]}}
 #[test]fn authenticated_roundtrip_and_no_plaintext(){let key=Zeroizing::new([7;32]);let salt=[3;16];let v=sample();let a=seal(&v,&key,&salt).unwrap();let b=seal(&v,&key,&salt).unwrap();assert!(a!=b);assert!(!a.windows(22).any(|w|w==b"never-write-this-secret"));let restored=open(&a,&key).unwrap();assert!(restored.entries[0].password==v.entries[0].password);assert!(open(&a,&Zeroizing::new([8;32])).is_err());}
 #[test]fn tampering_truncation_and_wrong_header_are_rejected(){let key=Zeroizing::new([7;32]);let original=seal(&sample(),&key,&[3;16]).unwrap();for index in [0,8,24,36,original.len()-1]{let mut changed=original.clone();changed[index]^=1;assert!(open(&changed,&key).is_err());}for len in [0,8,35,52,original.len()-1]{assert!(open(&original[..len],&key).is_err());}}
 #[test]fn argon2id_salt_and_password_change_keys(){let a=derive("a strong test master password",&[3;16]).unwrap();let b=derive("a strong test master password",&[4;16]).unwrap();let c=derive("a different test master password",&[3;16]).unwrap();assert!(a[..]!=b[..]);assert!(a[..]!=c[..]);assert!(derive("",&[3;16]).is_err());}
 #[test]fn atomic_encrypted_saves_keep_recovery_and_reject_external_changes(){let directory=tempfile::tempdir().unwrap();let path=directory.path().join("test.mtvault");let key=Zeroizing::new([7;32]);let first=seal(&sample(),&key,&[3;16]).unwrap();create(&path,&first).unwrap();assert!(create(&path,&first).is_err());let mut second_vault=sample();second_vault.entries[0].password="changed-secret".into();let second=seal(&second_vault,&key,&[3;16]).unwrap();commit(&path,&second,&fingerprint(&first)).unwrap();assert!(read(&path).unwrap()==second);assert!(read(&path.with_extension("mtvault.bak")).unwrap()==first);assert!(commit(&path,&first,&fingerprint(&first)).is_err());assert!(read(&path).unwrap()==second);#[cfg(unix)]{use std::os::unix::fs::PermissionsExt;assert_eq!(std::fs::metadata(path).unwrap().permissions().mode()&0o777,0o600);}}
 #[test]fn duplicates_and_oversized_fields_rejected(){let key=Zeroizing::new([7;32]);let mut v=sample();v.entries.push(v.entries[0].clone());assert!(seal(&v,&key,&[3;16]).is_err());v.entries.pop();v.entries[0].password="x".repeat(4097);assert!(seal(&v,&key,&[3;16]).is_err());}
}
