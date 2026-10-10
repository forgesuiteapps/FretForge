use std::{env,fs,path::PathBuf,process::Command};
fn u16le(b:&[u8],i:usize)->u16{u16::from_le_bytes(b[i..i+2].try_into().unwrap())}
fn u32le(b:&[u8],i:usize)->u32{u32::from_le_bytes(b[i..i+4].try_into().unwrap())}
fn resource(out:&mut Vec<u8>,kind:u16,id:u16,data:&[u8]){
 out.extend_from_slice(&(data.len() as u32).to_le_bytes());out.extend_from_slice(&32u32.to_le_bytes());
 for v in [0xffff,kind,0xffff,id]{out.extend_from_slice(&v.to_le_bytes());}
 out.extend_from_slice(&0u32.to_le_bytes());out.extend_from_slice(&0x1030u16.to_le_bytes());out.extend_from_slice(&1033u16.to_le_bytes());out.extend_from_slice(&[0;8]);
 out.extend_from_slice(data);while out.len()%4!=0{out.push(0);}
}
fn main(){
 println!("cargo:rerun-if-changed=assets/fretforge.ico");
 if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default()!="windows" {return;}
 let out=PathBuf::from(env::var("OUT_DIR").unwrap());let ico=fs::read("assets/fretforge.ico").unwrap();
 let count=u16le(&ico,4);let mut res=vec![];resource(&mut res,0,0,&[]);
 let mut group=vec![0,0,1,0];group.extend_from_slice(&count.to_le_bytes());
 for i in 0..count{let offset=6+i as usize*16;let length=u32le(&ico,offset+8) as usize;let data_offset=u32le(&ico,offset+12) as usize;
 resource(&mut res,3,i+1,&ico[data_offset..data_offset+length]);group.extend_from_slice(&ico[offset..offset+12]);group.extend_from_slice(&(i+1).to_le_bytes());}
 resource(&mut res,14,1,&group);let res_file=out.join("fretforge-icon.res");fs::write(&res_file,res).unwrap();
 let file=if env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default()=="gnu"{
 let obj=out.join("fretforge-icon.o");let status=Command::new("windres").arg("-Jres").arg("-Ocoff").arg("-i").arg(&res_file).arg("-o").arg(&obj).status().expect("GNU Windows builds require windres on PATH");assert!(status.success(),"Icon resource compilation failed");obj
 }else{res_file};
 println!("cargo:rustc-link-arg={}",file.display());
}
