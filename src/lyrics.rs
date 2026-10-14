//! LRC editing, chronological repair and Clone Hero phrase generation.
use serde::{Serialize,Deserialize};
use crate::engine::Options;
#[derive(Clone,Default,Serialize,Deserialize)]
pub struct Cue{pub track:usize,pub tick:f64,pub length:f64,pub line:usize,pub text:String}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub struct Repair{pub line:usize,pub text:String,pub original_ms:i64,pub repaired_ms:i64}
#[derive(Clone,Debug)]
pub struct Entry{pub ms:i64,pub text:String,pub line:usize,pub phrase:usize,pub enhanced:bool}
#[derive(Clone,Debug)]
pub struct Document{pub entries:Vec<Entry>,pub repairs:Vec<Repair>,pub enhanced:bool,pub metadata:Vec<String>}
pub fn stamp(ms:i64)->String{let sign=if ms<0{"-"}else{""};let ms=ms.unsigned_abs();format!("{sign}{:02}:{:02}.{:03}",ms/60000,(ms/1000)%60,ms%1000)}
pub fn parse(text:&str)->Result<Document,String>{
 static TIME:std::sync::OnceLock<regex::Regex>=std::sync::OnceLock::new();
 let time=TIME.get_or_init(||regex::Regex::new(r"\[(\d+):(\d{2})(?:[.:](\d{1,3}))?\]").unwrap());
 static WORD:std::sync::OnceLock<regex::Regex>=std::sync::OnceLock::new();
 let word=WORD.get_or_init(||regex::Regex::new(r"<(\d+):(\d{2})(?:[.:](\d{1,3}))?>").unwrap());
 static OFFSET:std::sync::OnceLock<regex::Regex>=std::sync::OnceLock::new();
 let offset_re=OFFSET.get_or_init(||regex::Regex::new(r"(?i)\[offset:([+-]?\d+)\]").unwrap());
 let offset=offset_re.captures(text).and_then(|c|c[1].parse::<i64>().ok()).unwrap_or(0);
 let ms=|c:&regex::Captures<'_>|->Result<i64,String>{let min=c[1].parse::<i64>().map_err(|_|"LRC minute value is too large.")?;let sec=c[2].parse::<i64>().unwrap();if sec>=60{return Err("LRC seconds must be between 00 and 59.".into())}let frac=c.get(3).map(|m|format!("{:0<3}",m.as_str()).parse::<i64>().unwrap()).unwrap_or(0);min.checked_mul(60000).and_then(|v|v.checked_add(sec*1000+frac)).and_then(|v|v.checked_add(offset)).filter(|v|v.unsigned_abs()<86_400_000).ok_or("Lyrics timestamps must be within 24 hours.".into())};
 let mut entries=Vec::new();let mut phrase=0;let mut enhanced=false;let mut metadata=Vec::new();
 for (line_no,line) in text.trim_start_matches('\u{feff}').lines().enumerate(){let captures:Vec<_>=time.captures_iter(line).collect();if captures.is_empty(){if line.trim().starts_with('[')&&!offset_re.is_match(line){metadata.push(line.to_string());}continue}let tail=&line[captures.last().unwrap().get(0).unwrap().end()..];let words:Vec<_>=word.captures_iter(tail).collect();
  if words.is_empty(){for c in captures{let txt=tail.trim();if !txt.is_empty(){entries.push(Entry{ms:ms(&c)?,text:txt.into(),line:line_no+1,phrase,enhanced:false});phrase+=1;}}}
  else {enhanced=true;for c in &captures{let delta=ms(c)?-ms(&captures[0])?;for (i,w) in words.iter().enumerate(){let start=w.get(0).unwrap().end();let end=words.get(i+1).map(|v|v.get(0).unwrap().start()).unwrap_or(tail.len());let txt=tail[start..end].trim();if !txt.is_empty(){entries.push(Entry{ms:ms(w)?+delta,text:txt.into(),line:line_no+1,phrase,enhanced:true});}}phrase+=1;}}
 }
 if entries.is_empty(){return Err("No timed lyrics found. Open or enter a standard or enhanced LRC file.".into())}
 // Preserve the written lyric order while repairing runs of backward enhanced timestamps.
 let mut repairs=Vec::new();if enhanced{let mut i=1;while i<entries.len(){if entries[i].ms<entries[i-1].ms{let previous=entries[i-1].ms;let mut next=i+1;while next<entries.len()&&entries[next].ms<=previous{next+=1;}if next==entries.len(){return Err(format!("Backward timestamps at line {} have no later lyric to anchor the repair. Add a following lyric with a later timestamp.",entries[i].line))}let count=next-i;let gap=entries[next].ms-previous;if gap<=count as i64{return Err("There is not enough time between the surrounding lyrics to place the backward timestamps. Move the following lyric later.".into())}for j in i..next{let repaired=previous+gap*(j-i+1) as i64/(count+1) as i64;repairs.push(Repair{line:entries[j].line,text:entries[j].text.clone(),original_ms:entries[j].ms,repaired_ms:repaired});entries[j].ms=repaired;}i=next;}else{i+=1;}}}
 entries.sort_by_key(|e|e.ms);entries.dedup_by(|a,b|a.ms==b.ms&&a.text==b.text&&a.phrase==b.phrase);
 Ok(Document{entries,repairs,enhanced,metadata})
}
/// Plain text receives provisional three-second line spacing for manual alignment.
pub fn import(text:&str,extension:&str)->Result<Document,String>{
 match extension.to_ascii_lowercase().as_str(){
 "lrc"=>parse(text),
 "txt"=>{if parse(text).is_ok(){return parse(text)}let lrc=text.trim_start_matches('\u{feff}').lines().map(str::trim).filter(|l|!l.is_empty()).enumerate().map(|(i,line)|format!("[{}]{line}\n",stamp(i as i64*3000))).collect::<String>();parse(&lrc)},
 "ttml"=>import_ttml(text),
_=>Err("Choose an .lrc, .ttml or .txt lyric file.".into())
 }
}
fn import_ttml(text:&str)->Result<Document,String>{
 use xml::reader::{EventReader,XmlEvent};
 struct Phrase{depth:usize,start:i64,chunks:Vec<(i64,String)>,timed:bool}
 let mut stack=vec![0i64];let mut paragraph:Option<Phrase>=None;
 let mut absolute=false;let mut line_only=false;let mut entries=Vec::new();let mut phrase=0;
 for event in EventReader::from_str(text){match event.map_err(|e|format!("Invalid TTML: {e}"))?{
  XmlEvent::StartElement{name,attributes,..}=>{
   if name.local_name=="tt"{if let Some(a)=attributes.iter().find(|a|a.name.namespace.as_deref()==Some("http://lrc.red/lyric-ttml-internal")&&a.name.local_name=="timing"){absolute=true;line_only=a.value.eq_ignore_ascii_case("Line")||a.value.eq_ignore_ascii_case("Phrase");}}
   let parent=*stack.last().unwrap();let begin=attributes.iter().find(|a|a.name.local_name=="begin").map(|a|ttml_time(&a.value)).transpose()?;
   // lrc.red uses absolute song timestamps, including bare decimal seconds.
   let at=if absolute{begin.unwrap_or(parent)}else{parent.checked_add(begin.unwrap_or(0)).ok_or("TTML time overflow.")?};
   if at>=86_400_000{return Err("TTML timestamps must be within 24 hours.".into());}stack.push(at);
   if name.local_name=="p"{paragraph=Some(Phrase{depth:stack.len(),start:at,chunks:Vec::new(),timed:false});}
   else if let Some(p)=&mut paragraph{if name.local_name=="span"&&begin.is_some(){p.timed=true;}if name.local_name=="br"{p.chunks.push((at," ".into()));}}
  },
  XmlEvent::Characters(s)|XmlEvent::CData(s)|XmlEvent::Whitespace(s)=>{if let Some(p)=&mut paragraph{p.chunks.push((*stack.last().unwrap(),s));}},
  XmlEvent::EndElement{..}=>{
   if paragraph.as_ref().is_some_and(|p|p.depth==stack.len()){
    let p=paragraph.take().unwrap();let enhanced=p.timed&&!line_only;
    if enhanced{
     // Whitespace separates words; contiguous timed syllables form one word,
     // whose attack is the first syllable's original timestamp.
     let mut word=String::new();let mut at=p.start;
     for (ms,chunk) in p.chunks{for ch in chunk.chars(){if ch.is_whitespace(){if !word.is_empty(){entries.push(Entry{ms:at,text:std::mem::take(&mut word),line:phrase+1,phrase,enhanced:true});}}else{if word.is_empty(){at=ms;}word.push(ch);}}}
     if !word.is_empty(){entries.push(Entry{ms:at,text:word,line:phrase+1,phrase,enhanced:true});}
    }else{let txt=p.chunks.into_iter().map(|(_,s)|s).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ");if !txt.is_empty(){entries.push(Entry{ms:p.start,text:txt,line:phrase+1,phrase,enhanced:false});}}
    phrase+=1;
   }stack.pop();
  },_=>{}
 }}
 if entries.is_empty(){return Err("No timed lyric phrases found in TTML.".into());}
 let enhanced=entries.iter().any(|e|e.enhanced);entries.sort_by_key(|e|e.ms);
 Ok(Document{entries,enhanced,repairs:Vec::new(),metadata:Vec::new()})
}
fn ttml_time(value:&str)->Result<i64,String>{
 let seconds=if let Some(v)=value.strip_suffix("ms"){v.parse::<f64>().ok().map(|n|n/1000.)}
 else if let Some(v)=value.strip_suffix('s'){v.parse::<f64>().ok()}
 else if let Some(v)=value.strip_suffix('m'){v.parse::<f64>().ok().map(|n|n*60.)}
 else if let Some(v)=value.strip_suffix('h'){v.parse::<f64>().ok().map(|n|n*3600.)}
 else if !value.contains(':'){value.parse::<f64>().ok()}
 else {let p:Vec<_>=value.split(':').collect();match p.as_slice(){
  [m,s]=>match(m.parse::<u64>(),s.parse::<f64>()){(Ok(m),Ok(s))if (0.0..60.0).contains(&s)=>Some(m as f64*60.+s),_=>None},
  [h,m,s]=>match(h.parse::<u64>(),m.parse::<u64>(),s.parse::<f64>()){(Ok(h),Ok(m),Ok(s))if m<60&&(0.0..60.0).contains(&s)=>Some(h as f64*3600.+m as f64*60.+s),_=>None},_=>None
 }};
 seconds.filter(|s|s.is_finite()&&*s>=0.&&*s<86400.).map(|s|(s*1000.).round() as i64).ok_or_else(||format!("Unsupported TTML time '{value}'. Use mm:ss.mmm, hh:mm:ss.mmm or h/m/s/ms offsets; frame and tick times need conversion."))
}
pub fn load(o:&Options)->Result<Document,String>{if let Some(text)=&o.lyric_text{return parse(text)}let p=o.lyrics.as_ref().ok_or("Open an LRC, TTML or TXT file to add lyrics.")?;let text=std::fs::read_to_string(p).map_err(|e|format!("Cannot read lyrics: {e}"))?;import(&text,p.extension().and_then(|e|e.to_str()).unwrap_or(""))}
impl Document {
 pub fn lrc(&self,offset:i64)->String{let mut groups:std::collections::BTreeMap<usize,Vec<&Entry>>=std::collections::BTreeMap::new();for e in &self.entries{groups.entry(e.phrase).or_default().push(e);}let mut lines=Vec::new();for group in groups.values(){let start=(group[0].ms+offset).max(0);let mut line=format!("[{}]",stamp(start));if group[0].enhanced{for e in group{line.push_str(&format!("<{}>{} ",stamp((e.ms+offset).max(0)),e.text));}}else{line.push_str(&group[0].text);}lines.push((start,line.trim_end().to_string()));}lines.sort_by_key(|p|p.0);{let mut out=self.metadata.iter().map(|l|l.clone()+"\n").collect::<String>();out.extend(lines.into_iter().map(|(_,l)|l+"\n"));out}}
 pub fn review(&self,history:&[Repair])->Vec<Repair>{let mut out=history.to_vec();for r in &self.repairs{if !out.contains(r){out.push(r.clone());}}out}
}
pub fn from_guitar_pro(score:&crate::engine::Score,o:&Options,track:usize,verse:usize)->Result<String,String>{
 let timeline=crate::timing::Timeline::new(score,o);let mut cues:Vec<_>=score.vocal_lyrics.iter().filter(|c|c.track==track&&c.line==verse).collect();cues.sort_by(|a,b|a.tick.total_cmp(&b.tick));if cues.is_empty(){return Err("This track/verse contains no Guitar Pro lyrics.".into())}let mut out=String::new();let mut text=String::new();let mut start=0.;let mut end=0.;let mut join=false;
 for c in cues{if !text.is_empty()&&(c.tick-end>=score.ppq as f64||c.tick-start>=score.ppq as f64*8.){let ms=(timeline.seconds(start+o.note_shift_ticks(score) as f64)*1000.).round() as i64;out.push_str(&format!("[{}]{}\n",stamp(ms),text));text.clear();join=false;}if text.is_empty(){start=c.tick;}if !text.is_empty()&&!join{text.push(' ')}join=c.text.ends_with('-');text.push_str(c.text.trim_end_matches('-'));end=c.tick+c.length;}
 if !text.is_empty(){let ms=(timeline.seconds(start+o.note_shift_ticks(score) as f64)*1000.).round() as i64;out.push_str(&format!("[{}]{}\n",stamp(ms),text));}Ok(out)
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn line_ttml_keeps_bare_second_timing_and_whole_phrases(){let text=r#"<tt xmlns:lrc="http://lrc.red/lyric-ttml-internal" lrc:timing="Line"><body><div begin="22.295"><p begin="22.295" end="26.948">A whole line</p><p begin="29.483" end="31.727"><span>Another</span> line</p></div></body></tt>"#;let d=import(text,"ttml").unwrap();assert!(!d.enhanced);assert_eq!(d.entries.len(),2);assert_eq!(d.entries[0].ms,22295);assert_eq!(d.entries[1].ms,29483);assert_eq!(d.entries[0].text,"A whole line");assert_eq!(d.entries[1].text,"Another line");assert_ttml_chart(&d);}
 fn assert_ttml_chart(d:&Document){use crate::engine::{self,Score,Track,Note,Tempo};let s=Score{ppq:1000,end:300000.,tempos:vec![Tempo{tick:0.,bpm:60.}],tracks:vec![Track{index:0,percussion:true,notes:vec![Note{tick:0.,pitch:36,..Default::default()}],..Default::default()}],..Default::default()};let o=Options{bpm:60.,tracks:[Some(0),None,None,None,None,None,None],lyric_text:Some(d.lrc(0)),..Default::default()};let (chart,_)=engine::build_chart(&s,&o).unwrap();for e in &d.entries{assert!(chart.contains(&format!("{} = E \"lyric {}\"",e.ms,e.text.replace('"',"'"))),"Missing event at {}",e.ms);}let phrases=d.entries.iter().map(|e|e.phrase).collect::<std::collections::BTreeSet<_>>().len();assert_eq!(chart.matches("phrase_start").count(),phrases);assert_eq!(chart.matches("phrase_end").count(),phrases);}
 #[test]fn supplied_line_example_charts_each_phrase_when_requested(){if let Ok(path)=std::env::var("FRETFORGE_TTML_LINE_TEST_FILE"){let text=std::fs::read_to_string(path).unwrap();let d=import(&text,"ttml").unwrap();assert_eq!(d.entries.len(),31);assert!(!d.enhanced);assert_eq!(d.entries[0].ms,22295);assert_eq!(d.entries[1].ms,29483);assert_ttml_chart(&d);}}
 #[test]fn lyric_ttml_short_clocks_are_absolute_and_syllables_stay_joined(){let text=r#"<tt xmlns="http://www.w3.org/ns/ttml" xmlns:lrc="http://lrc.red/lyric-ttml-internal" lrc:timing="Word"><body><div begin="0:15.355"><p begin="0:15.355"><span begin="0:15.355">Some</span><span begin="0:16.000">one</span> <span begin="0:16.500">sings</span></p><p begin="1:06.720">Next line</p></div></body></tt>"#;let d=import(text,"ttml").unwrap();assert_eq!(d.entries[0].ms,15355);assert_eq!(d.entries[0].text,"Someone");assert_eq!(d.entries[1].text,"sings");assert_eq!(d.entries[1].ms,16500);assert_eq!(d.entries[2].ms,66720);assert_eq!(d.entries[0].phrase,d.entries[1].phrase);assert_ne!(d.entries[1].phrase,d.entries[2].phrase);assert_eq!(ttml_time("3:24.188").unwrap(),204188);assert!(ttml_time("0:60.000").is_err());assert!(ttml_time("-1:15.000").is_err());}
 #[test]fn supplied_ttml_example_imports_when_requested(){if let Ok(path)=std::env::var("FRETFORGE_TTML_TEST_FILE"){let text=std::fs::read_to_string(path).unwrap();let d=import(&text,"ttml").unwrap();assert_eq!(d.entries.iter().map(|e|e.phrase).collect::<std::collections::BTreeSet<_>>().len(),41);assert!(d.enhanced);assert!(d.entries.len()>41);assert_eq!(d.entries.first().unwrap().ms,15355);assert_eq!(d.entries.iter().filter(|e|e.phrase==40).map(|e|e.ms).min(),Some(204188));assert_eq!(d.entries[1].ms,15615);assert_eq!(d.entries[2].ms,15816);assert_ttml_chart(&d);assert!(parse(&d.lrc(0)).is_ok());}}
 #[test]fn imports_ttml_namespaces_entities_and_nested_timing(){let d=import(r#"<tt xmlns="http://www.w3.org/ns/ttml"><body begin="1s"><div><p begin="00:00:02.500"><span>Hello &amp;</span><br/>world</p></div></body></tt>"#,"ttml").unwrap();assert_eq!(d.entries[0].ms,3500);assert_eq!(d.entries[0].text,"Hello & world");assert!(import("<tt>","ttml").is_err());}
 #[test]fn plain_text_has_editable_provisional_timing(){let d=import("\u{feff}First\n\nSecond","TXT").unwrap();assert_eq!(d.entries[1].ms,3000);assert_eq!(d.entries[0].text,"First");assert!(import("","txt").is_err());assert_eq!(import("[00:07]Timed","txt").unwrap().entries[0].ms,7000);}
 #[test]fn backward_runs_are_evenly_spaced_without_reordering_words(){let d=parse("[00:10.000]<00:10.000>A <00:08.000>B <00:07.000>C <00:16.000>D").unwrap();assert_eq!(d.entries.iter().map(|e|e.ms).collect::<Vec<_>>(),vec![10000,12000,14000,16000]);assert_eq!(d.entries.iter().map(|e|e.text.as_str()).collect::<Vec<_>>(),vec!["A","B","C","D"]);assert_eq!(d.repairs.len(),2);assert_eq!(d.repairs[0].original_ms,8000);assert_eq!(d.repairs[0].repaired_ms,12000);assert!(parse(&d.lrc(0)).unwrap().repairs.is_empty());}
 #[test]fn cross_line_repairs_use_the_next_lyric_and_preserve_phrases(){let d=parse("[00:10]<00:10>A <00:08>B\n[00:14]<00:14>C <00:15>D").unwrap();assert_eq!(d.entries[1].ms,12000);assert_eq!(d.entries[1].phrase,0);assert_eq!(d.entries[2].phrase,1);assert!(d.lrc(500).contains("<00:12.500>B"));}
 #[test]fn standard_multiple_timestamps_offsets_and_order_are_preserved(){let d=parse("[offset:100]\n[00:03.000][00:01.50]Hello").unwrap();assert_eq!(d.entries.iter().map(|e|e.ms).collect::<Vec<_>>(),vec![1600,3100]);assert!(d.repairs.is_empty());assert!(!d.enhanced);}
 #[test]fn missing_next_anchor_and_too_small_gaps_are_actionable(){assert!(parse("[00:10]<00:10>A <00:08>B").unwrap_err().contains("no later lyric"));assert!(parse("[00:10.000]<00:10.000>A <00:08.000>B <00:07.000>C <00:10.001>D").unwrap_err().contains("not enough time"));}
}

pub fn snap_seconds(seconds:f64,score:Option<&crate::engine::Score>,o:&crate::engine::Options)->f64{
 if !o.lyric_snap{return seconds.max(0.);}
 let division=if [1,2,4,8,16].contains(&o.lyric_snap_division){o.lyric_snap_division as f64}else{4.};
 if let Some(score)=score{let timeline=crate::timing::Timeline::new(score,o);let beat=timeline.tick(seconds)/score.ppq as f64;timeline.seconds((beat*division).round()/division*score.ppq as f64)}
 else{let beat=seconds*o.bpm.max(0.001)/60.;(beat*division).round()/division*60./o.bpm.max(0.001)}
}

#[cfg(test)]mod snap_tests{
 use super::*;use crate::engine::{Options,Score,Tempo};
 #[test]fn sixteenth_snap_follows_tempo_changes_and_preserves_unsnapped_values(){
 let s=Score{ppq:480,tempos:vec![Tempo{tick:0.,bpm:120.},Tempo{tick:960.,bpm:240.}],..Default::default()};let mut o=Options{lyric_snap:true,lyric_snap_division:4,..Default::default()};
 assert!((snap_seconds(0.18,Some(&s),&o)-0.125).abs()<1e-9);assert!((snap_seconds(1.08,Some(&s),&o)-1.0625).abs()<1e-9);
 o.lyric_snap_division=16;assert!((snap_seconds(1.019,Some(&s),&o)-1.015625).abs()<1e-9);
 o.lyric_snap=false;assert_eq!(snap_seconds(1.019,Some(&s),&o),1.019);
 o.lyric_snap=true;o.lyric_snap_division=4;o.bpm=60.;assert_eq!(snap_seconds(0.39,None,&o),0.5);
 }
}
