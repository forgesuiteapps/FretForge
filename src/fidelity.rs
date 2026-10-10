//! Auditable source-to-target decisions; source pitches never define global lanes.
use crate::engine::{Gem,Options,Score,Track};
use std::collections::{BTreeMap,BTreeSet};
use serde_json::{json,Value};
pub fn report(score:&Score,o:&Options,track:&Track,notes:&[Gem],six:bool)->Value{
 let shift=o.note_shift_ticks(score);let mut source:BTreeMap<i64,Vec<&crate::engine::Note>>=BTreeMap::new();
 for n in &track.notes{source.entry(n.tick.round() as i64).or_default().push(n);}
 let mut target:BTreeMap<i64,Vec<&Gem>>=BTreeMap::new();
 for n in notes.iter().filter(|n|crate::engine::playable(n.lane)){target.entry(n.tick-shift).or_default().push(n);}
 let mut decisions=vec![];let mut events=vec![];let mut prior:Option<(i64,i32,i32)>=None;
 for (&tick,voices) in &source{
  let gems=target.get(&tick).cloned().unwrap_or_default();let pitches=voices.iter().map(|n|n.pitch).collect::<BTreeSet<_>>();
  let selected=crate::pitched::voicing(&pitches.iter().copied().collect::<Vec<_>>());
  let mut add=|kind:&str,detail:String|decisions.push(json!({"tick":tick+shift,"source_tick":tick,"kind":kind,"detail":detail}));
  let onsets=voices.iter().map(|n|n.tick.to_bits()).collect::<BTreeSet<_>>();
  if onsets.len()>1{add("rounding_collision","Distinct source onsets share one output tick; inspect grace or rolled attacks.".into());}
  if voices.iter().any(|n|n.tick!=tick as f64){add("timing_rounding","Absolute onset rounded to nearest tick; no grid snapping or accumulated drift.".into());}
  if gems.len()<pitches.len(){add("chord_reduction",format!("{} distinct pitches represented by {} buttons. Root, characteristic inner harmony and top voice prioritized; source pitches {:?}, retained voice candidates {:?}.",pitches.len(),gems.len(),pitches,selected));}
  if voices.len()>pitches.len(){add("unison_doubling",format!("{} source voices share {} pitches; simultaneous unisons share a gem.",voices.len(),pitches.len()));}
  if voices.iter().any(|n|!n.picked&&!n.hopo&&!n.tap){add("articulation_inference","Unspecified technique conservatively requires a strum. This is an inference, not evidence of picking.".into());}
  if six&&o.ghl_octave_folding{add("octave_folding_enabled","User enabled octave equivalence; original pitches retained in this report, mapped contour may change.".into());}
  if voices.iter().any(|n|n.hopo&&n.picked){add("conflicting_articulation","Written legato and explicit picking conflict; conservative strum chosen. Verify transcription against audio.".into());}
  if voices.iter().any(|n|n.source.is_null()){add("missing_provenance","Playback event has no matching written technique metadata; review source articulation and sounding duration.".into());}
  for n in voices{
   let raw=n.length.floor() as i64;
   if n.length!=raw as f64{add("duration_rounding",format!("Pitch {} release rounded down by {:.4} ticks.",n.pitch,n.length-raw as f64));}
   if n.dead||n.palm_mute||n.staccato||n.tremolo{if raw>0{add("detached_release",format!("Pitch {} has detached/dead/muted/tremolo articulation; tail suppressed.",n.pitch));}}
   else if let Some(index)=selected.iter().position(|p|*p==n.pitch){
    // Six-fret encoding is not numerically ordered by physical voice position.
    let mut ordered=gems.clone();if six{ordered.sort_by_key(|g|crate::ghl::preview_lane(g.lane));}
    if let Some(g)=ordered.get(index){if g.length<raw{add("sustain_shortening",format!("Pitch {} source duration {} ticks → {}. Short notation tail policy, same-button/open conflict or target voice reduction; review ringing against recording.",n.pitch,raw,g.length));}}
   }
   if n.bend||n.slide||n.source.get("vibrato").and_then(Value::as_i64).unwrap_or(0)>0||n.source.get("harmonic").and_then(Value::as_i64).unwrap_or(0)>0||n.source.get("let_ring")==Some(&json!(true)){
    add("expressive_detail",format!("Pitch {} retains its attack/release; bend, slide, vibrato, harmonic or ringing expression has no equivalent gameplay control. Source metadata retained.",n.pitch));
   }
  }
  if pitches.len()==1&&gems.len()==1{let pitch=*pitches.first().unwrap();let lane=gems[0].lane;
   let position=if six{crate::ghl::preview_lane(lane).map(|i|if i<6{i as i32}else{-1}).unwrap_or(-1)}else{lane};
   if let Some((_,p,l))=prior{let previous_position=if six{crate::ghl::preview_lane(l).map(|i|if i<6{i as i32}else{-1}).unwrap_or(-1)}else{l};
    if position>=0&&previous_position>=0&&pitch!=p&&(pitch-p)*(position-previous_position)<=0{add("position_reset_or_contour_compromise",format!("Pitch {} → {}, button {} → {}. Contextual lane reuse/physical row movement; no attack removed.",p,pitch,l,lane));}
    if pitch==p&&lane!=l{add("anchor_change",format!("Repeated pitch {} changes button at a phrase/reset boundary: {} → {}.",pitch,l,lane));}
   }prior=Some((tick,pitch,lane));
  }else{prior=None;}
  let modifiers=notes.iter().filter(|n|n.tick==tick+shift&&(n.lane==5||n.lane==6)).map(|n|n.lane).collect::<Vec<_>>();
  events.push(json!({"source_tick":tick,"output_tick":tick+shift,"source_voices":voices,"target_gems":gems.iter().map(|g|json!({"lane":g.lane,"length":g.length})).collect::<Vec<_>>(),"modifiers":modifiers}));
 }
 let expected=source.keys().copied().collect::<BTreeSet<_>>();let actual=target.keys().copied().collect::<BTreeSet<_>>();
 json!({"source_attack_events":track.notes.iter().map(|n|n.tick.to_bits()).collect::<BTreeSet<_>>().len(),"rounded_source_attack_events":expected.len(),"target_attack_events":actual.len(),"onsets_match_after_rounding_and_shared_offset":expected==actual,"source_note_voices":track.notes.len(),"source_metadata":score.source_metadata,"decisions":decisions,"events":events,
 "target":"Clone Hero .chart; GHL W1/W2/W3=0/1/2, B1/B2/B3=3/4/8, OPEN=7, forced-toggle=5, tap=6",
 "house_policy":"At most three significant chord voices; six-fret three-column grips with optional simple two-button barre. This is an ergonomic policy, not a universal engine chord limit. Expert tails retain longer and explicitly ringing notes; ordinary quarter-note or shorter notation tails are omitted by default, with a user override; detached tails are suppressed. Lower difficulties are intentionally reduced.",
 "review_required":"No automatic audio transcription or controller playtest. Compare uncertain techniques and ringing durations against the recording; verify in your Clone Hero version."})
}

#[cfg(test)]mod tests{
 use super::*;use crate::engine::{self,Note};
 fn n(t:f64,p:i32)->Note{Note{tick:t,pitch:p,..Default::default()}}
 fn setup(notes:Vec<Note>)->(Score,Options){(Score{ppq:480,end:10000.,tracks:vec![Track{index:0,name:"Source".into(),notes,percussion:false}],..Default::default()},Options{short_sustains:true,tracks:[None,Some(0),Some(0),Some(0),Some(0),Some(0),Some(0)],ghl_open_pedals:false,..Default::default()})}
 #[test]fn all_six_roles_preserve_long_irregular_expert_runs_and_report_resets(){
 let (s,o)=setup((0..30).map(|i|Note{picked:true,length:17.,..n((i*73) as f64,40+i)}).collect());
 for role in 1..7{let gems=engine::gems(&s,&o,role,3).unwrap();let f=report(&s,&o,&s.tracks[0],&gems,role>=4);assert_eq!(f["source_attack_events"],30);assert_eq!(f["target_attack_events"],30);assert_eq!(f["onsets_match_after_rounding_and_shared_offset"],true);assert!(f["decisions"].as_array().unwrap().iter().any(|d|d["kind"]=="position_reset_or_contour_compromise"));assert!(!gems.iter().any(|g|g.lane==7));}
 }
 #[test]fn five_fret_narrow_transposed_phrases_use_compact_identical_shapes(){
 let notes=[(0,0),(4000,12)].into_iter().flat_map(|(start,transpose)|[60,61,62,61,60].into_iter().enumerate().map(move|(i,p)|n((start+i*120) as f64,p+transpose))).collect();let (s,o)=setup(notes);let g=engine::gems(&s,&o,1,3).unwrap();let lanes=g.iter().filter(|g|engine::playable(g.lane)).map(|g|g.lane).collect::<Vec<_>>();assert_eq!(lanes[..5],lanes[5..]);assert!(lanes.iter().max().unwrap()-lanes.iter().min().unwrap()<=2);
 }
 #[test]fn literal_releases_picked_strums_and_same_lane_taps_survive_all_roles(){
 let (s,o)=setup(vec![Note{picked:true,length:37.,..n(0.,60)},Note{picked:true,length:52.,..n(73.,62)},Note{tap:true,length:14.,..n(146.,62)},Note{tap:true,length:15.,..n(219.,62)}]);
 for role in 1..7{let g=engine::gems(&s,&o,role,3).unwrap();assert_eq!(g.iter().find(|g|g.tick==0&&engine::playable(g.lane)).unwrap().length,37);assert!(g.iter().any(|g|g.tick==73&&g.lane==5));for tick in [146,219]{assert!(g.iter().any(|g|g.tick==tick&&g.lane==6));}}
 }
 #[test]fn significant_triads_and_independent_releases_are_not_lost_at_speed(){
 let (s,o)=setup(vec![Note{length:37.,..n(0.,60)},Note{length:90.,..n(0.,64)},Note{length:150.,..n(0.,67)},n(73.,72)]);
 for role in 1..7{let g=engine::gems(&s,&o,role,3).unwrap();let mut chord=g.iter().filter(|g|g.tick==0&&engine::playable(g.lane)).collect::<Vec<_>>();if role>=4{chord.sort_by_key(|g|crate::ghl::preview_lane(g.lane));}assert_eq!(chord.len(),3);assert_eq!(chord[0].length,37);assert_ne!(chord[0].length,chord[2].length);}
 }
 #[test]fn quarter_notes_are_short_by_default_with_explicit_ringing_and_user_override(){
 let (s,mut o)=setup(vec![Note{length:480.,..n(0.,60)},Note{length:960.,..n(1920.,62)},Note{length:240.,source:json!({"let_ring":true}),..n(3840.,64)}]);o.short_sustains=false;
 for role in 1..7{let g=engine::gems(&s,&o,role,3).unwrap();let tail=|tick|g.iter().find(|g|g.tick==tick&&engine::playable(g.lane)).unwrap().length;assert_eq!(tail(0),0);assert_eq!(tail(1920),960);assert_eq!(tail(3840),240);}
 o.short_sustains=true;for role in 1..7{assert_eq!(engine::gems(&s,&o,role,3).unwrap().iter().find(|g|g.tick==0&&engine::playable(g.lane)).unwrap().length,480);}
 }
 #[test]fn report_exposes_chord_losses_rounding_collision_and_inference(){
 let (s,o)=setup(vec![n(0.1,60),n(0.2,64),n(0.1,67),n(0.1,70),n(0.1,72)]);let (_,r)=engine::build_chart(&s,&o).unwrap();for part in r["literal_fidelity"].as_array().unwrap(){let kinds=part["fidelity"]["decisions"].as_array().unwrap().iter().map(|d|d["kind"].as_str().unwrap()).collect::<Vec<_>>();assert!(kinds.contains(&"rounding_collision"));assert!(kinds.contains(&"chord_reduction"));assert!(kinds.contains(&"articulation_inference"));}
 }
}
