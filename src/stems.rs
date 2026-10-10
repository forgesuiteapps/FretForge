use std::{collections::BTreeMap,fs,path::{Path,PathBuf},process::{Command,Stdio}};
use crate::engine::Options;
#[derive(Clone,serde::Serialize,serde::Deserialize)]
pub struct Separated {pub source:PathBuf,pub backing:PathBuf,pub tracks:BTreeMap<String,PathBuf>}
impl Separated {
 pub fn validate(&self,o:&Options)->Result<(),String>{
  if o.recording_source().as_ref()!=Some(&self.source){return Err("The recording changed after stem separation. Clear the separated stems or separate the new recording in Extra media.".into())}
  for (label,path) in &self.tracks{if o.extras.get(label)!=Some(path){return Err(format!("{label} changed after separation. Clear the separated set before using a different stem so the backing mix remains correct."))}}
  if !self.backing.is_file(){return Err("Separated backing audio is missing. Separate again or clear the separated set.".into())}Ok(())
 }
 pub fn assign(self,o:&mut Options){for key in ["Kick stem","Snare stem","Toms stem","Cymbals stem","Rhythm stem","Keys stem"]{o.extras.remove(key);}for (label,path) in &self.tracks{o.extras.insert(label.clone(),path.clone());}o.separated=Some(self);}
}
pub fn clear(o:&mut Options){if let Some(s)=o.separated.take(){for (label,path) in s.tracks{if o.extras.get(&label)==Some(&path){o.extras.remove(&label);}}}}
fn command(executable:&Path)->Command{let mut c=Command::new(executable);#[cfg(windows)]{use std::os::windows::process::CommandExt;c.creation_flags(0x08000000);}c}
fn run(c:&mut Command,log:&Path)->Result<(),String>{let f=fs::OpenOptions::new().create(true).append(true).open(log).map_err(|e|e.to_string())?;let status=c.stdout(Stdio::from(f.try_clone().map_err(|e|e.to_string())?)).stderr(Stdio::from(f)).status().map_err(|e|format!("Could not start stem tools: {e}. Choose an installed Python executable in Extra media."))?;if !status.success(){let tail=log_tail(log);return Err(format!("Stem tools failed. {tail}\nFull details: {}",log.display()))}Ok(())}
pub fn log_tail(log:&Path)->String{use std::io::{Read,Seek,SeekFrom};let Ok(mut f)=fs::File::open(log) else{return String::new()};let len=f.metadata().map(|m|m.len()).unwrap_or(0);let _=f.seek(SeekFrom::Start(len.saturating_sub(3000)));let mut b=Vec::new();let _=f.read_to_end(&mut b);String::from_utf8_lossy(&b).lines().rev().take(3).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n")}
pub fn setup(python:&Path,folder:&Path,log:&Path)->Result<PathBuf,String>{
 let env=folder.join("FretForge-stem-tools");
 run(command(python).args(["-c","import sys; assert (3,10) <= sys.version_info[:2] <= (3,11), 'Stem setup needs Python 3.10 or 3.11'"]),log)?;
 run(command(python).arg("-m").arg("venv").arg(&env),log)?;
 let executable=env.join(if cfg!(windows){"Scripts/python.exe"}else{"bin/python"});
 run(command(&executable).args(["-m","pip","install","torch==2.5.1","torchaudio==2.5.1","--index-url","https://download.pytorch.org/whl/cpu"]),log)?;
 run(command(&executable).args(["-m","pip","install","demucs==4.0.1","numpy<2","soundfile"]),log)?;Ok(executable)
}
pub fn separate(python:&Path,source:&Path,folder:&Path,log:&Path)->Result<Separated,String>{
 if !source.is_file(){return Err("Choose an existing song recording or music video first.".into())}
 run(command(python).args(["-c","import demucs, torch, torchaudio, soundfile"]),log).map_err(|e|format!("Install stem tools first, or choose Python with Demucs already installed. {e}"))?;
 let root=folder.join(format!("FretForge-stems-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir(&root).map_err(|e|e.to_string())?;
 // Decode container audio once so video and uncommon codecs work identically.
 let input=root.join("recording.wav");
 run(crate::audio::ffmpeg().arg("-i").arg(source).args(["-map","0:a:0","-vn","-ac","2","-ar","44100","-c:a","pcm_f32le","-n"]).arg(&input),log)?;
 let mut c=command(python);c.args(["-m","demucs","-n","htdemucs_6s","-d","cpu","--float32","--clip-mode","clamp","--shifts","1","-o"]).arg(&root).arg(&input);
 // Demucs' Windows decoder finds our bundled FFmpeg without changing global PATH.
 let vendor=crate::engine::asset_root().join("vendor");let mut paths=vec![vendor];if let Some(path)=std::env::var_os("PATH"){paths.extend(std::env::split_paths(&path));}if let Ok(path)=std::env::join_paths(paths){c.env("PATH",path);}c.env("PYTHONUNBUFFERED","1");run(&mut c,log)?;
 finish(source,&root.join("htdemucs_6s/recording"),log)
}
fn finish(source:&Path,dir:&Path,log:&Path)->Result<Separated,String>{
 let mut tracks=BTreeMap::new();let mut duration=None;
 for (label,file) in [("Vocals stem","vocals"),("Guitar stem","guitar"),("Bass stem","bass"),("Drums stem","drums"),("remaining accompaniment","other"),("remaining piano","piano")]{
  let p=dir.join(format!("{file}.wav"));let wav=hound::WavReader::open(&p).map_err(|e|format!("Missing or unreadable separated {file} audio: {e}"))?;
  let shape=(wav.duration(),wav.spec().sample_rate,wav.spec().channels);if let Some(expected)=duration{if shape!=expected{return Err("Separated tracks have unequal lengths or formats. Nothing was assigned.".into())}}else{duration=Some(shape)}
  if file!="other"&&file!="piano"{tracks.insert(label.into(),p);}
 }
 let backing=dir.join("backing.wav");run(crate::audio::ffmpeg().arg("-i").arg(dir.join("other.wav")).arg("-i").arg(dir.join("piano.wav")).args(["-filter_complex","[0:a][1:a]amix=inputs=2:duration=longest:normalize=0","-c:a","pcm_f32le","-n"]).arg(&backing),log)?;
 Ok(Separated{source:source.into(),backing,tracks})
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn completed_stems_assign_atomically_keep_residual_mix_and_reject_stale_sources(){
  let root=std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join(format!("stem-test-{}",std::process::id()));fs::create_dir_all(&root).unwrap();let log=root.join("test.log");
  for (i,name) in ["vocals","guitar","bass","drums","other","piano"].iter().enumerate(){let mut w=hound::WavWriter::create(root.join(format!("{name}.wav")),hound::WavSpec{channels:2,sample_rate:8000,bits_per_sample:32,sample_format:hound::SampleFormat::Float}).unwrap();for _ in 0..16000{w.write_sample((i+1) as f32*0.01).unwrap()}w.finalize().unwrap();}
  let source=root.join("original.wav");fs::copy(root.join("vocals.wav"),&source).unwrap();let s=finish(&source,&root,&log).unwrap();let mut o=Options{audio:Some(source.clone()),..Default::default()};o.extras.insert("Rhythm stem".into(),source.clone());s.assign(&mut o);assert_eq!(o.extras.len(),4);assert!(!o.extras.contains_key("Rhythm stem"));o.separated.as_ref().unwrap().validate(&o).unwrap();let mut wav=hound::WavReader::open(&o.separated.as_ref().unwrap().backing).unwrap();assert_eq!(wav.duration(),8000);assert!((wav.samples::<f32>().next().unwrap().unwrap()-0.11).abs()<0.00001);
  o.audio=Some(root.join("different.wav"));assert!(o.separated.as_ref().unwrap().validate(&o).is_err());o.audio=Some(source);o.extras.remove("Guitar stem");assert!(o.separated.as_ref().unwrap().validate(&o).is_err());clear(&mut o);assert!(o.separated.is_none());assert!(o.extras.is_empty());let _=fs::remove_dir_all(root);
 }
}
