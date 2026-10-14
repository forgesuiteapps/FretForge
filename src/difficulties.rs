//! Generate lower difficulties without rewriting the original chart sections.
use std::collections::{BTreeMap, BTreeSet};

pub struct Generated { pub chart: String, pub counts: Vec<(String, [usize; 3])> }

pub fn generate(text: &str) -> Result<Generated, String> {
    let section = regex::Regex::new(r"(?ms)^\s*\[([^\]\r\n]+)\]\s*\{([^}]*)\}").unwrap();
    let note = regex::Regex::new(r"^\s*(\d+)\s*=\s*N\s+(\d+)\s+(\d+)\s*$").unwrap();
    let resolution = regex::Regex::new(r"(?m)^\s*Resolution\s*=\s*(\d+)").unwrap()
        .captures(text).and_then(|c|c[1].parse::<i64>().ok()).filter(|v|*v>0)
        .ok_or("Chart needs a positive Song Resolution.")?;
    let sections: Vec<_> = section.captures_iter(text).collect();
    let names: BTreeSet<_> = sections.iter().map(|c|c[1].trim().to_string()).collect();
    if names.len()!=sections.len() { return Err("Duplicate chart sections are not supported.".into()); }
    let mut chart=text.to_string();
    let mut counts=Vec::new();
    for c in sections {
        let Some(part)=c[1].trim().strip_prefix("Expert") else {continue};
        if !crate::engine::SUFFIXES.contains(&part) {return Err(format!("Unsupported Expert part: {part}"));}
        if ["Easy","Medium","Hard"].iter().any(|t|names.contains(&format!("{t}{part}"))) {
            return Err(format!("{part} already has lower difficulties. Select an Expert-only chart."));
        }
        let drums=part=="Drums";
        let ghl=part.starts_with("GHL");
        let mut groups:BTreeMap<i64,Vec<(i32,i64)>>=BTreeMap::new();
        let mut phrases=Vec::new();
        for line in c[2].lines().filter(|l|!l.trim().is_empty()) {
            if let Some(n)=note.captures(line) {
                let tick=n[1].parse::<i64>().map_err(|_|"Note tick is too large.")?;
                let lane=n[2].parse::<i32>().map_err(|_|"Note lane is too large.")?;
                let length=n[3].parse::<i64>().map_err(|_|"Sustain is too large.")?;
                let playable=if drums {(0..=4).contains(&lane)||lane==32} else {(0..=4).contains(&lane)||lane==7||ghl&&lane==8};
                if playable||drums&&(66..=68).contains(&lane) {groups.entry(tick).or_default().push((lane,length));}
            } else if line.contains("= S ")||line.contains("= E ") {phrases.push(line.trim().to_string());}
            else {return Err(format!("Unrecognized event in Expert{part}: {}",line.trim()));}
        }
        if groups.is_empty(){continue;}
        let mut part_counts=[0;3];
        for tier in 0..3 {
            let spacing=(resolution/[1,2,4][tier]).max(1);
            let mut retained=Vec::new();let mut last=None;
            for (&tick,notes) in &groups {
                if last.is_some_and(|l|tick-l<spacing){continue;}
                let mut playable:Vec<_>=notes.iter().copied().filter(|(l,_)|*l<64).collect();
                playable.sort_by_key(|n|n.0);playable.dedup_by_key(|n|n.0);
                if drums {for n in &mut playable{if n.0==32{n.0=0;}n.1=0;}}
                else if !ghl {for n in &mut playable {n.0=if n.0==7{0}else{n.0.min([2,3,4][tier])};}}
                // Open notes cannot share a chord with fretted notes.
                if !drums&&playable.iter().any(|n|n.0==7){playable.retain(|n|n.0==7);}
                playable.sort_by_key(|n|n.0);playable.dedup_by_key(|n|n.0);
                playable.truncate(if drums{[2,2,3][tier]}else{[1,2,3][tier]});
                if playable.is_empty(){continue;}
                part_counts[tier]+=playable.len();
                for n in &playable{retained.push((tick,n.0,n.1));}
                if drums {for &(lane,_) in notes {if lane>=64&&playable.iter().any(|n|n.0==lane-64){retained.push((tick,lane,0));}}}
                last=Some(tick);
            }
            let starts:Vec<_>=retained.iter().map(|n|n.0).collect::<BTreeSet<_>>().into_iter().collect();
            let mut events=Vec::new();
            for (tick,lane,length) in retained {
                let length=if length<resolution/2{0}else{starts.iter().find(|&&t|t>tick).map(|t|length.min((t-tick-1).max(0))).unwrap_or(length)};
                events.push((tick,format!("  {tick} = N {lane} {length}")));
            }
            for phrase in &phrases {let tick=phrase.split_whitespace().next().and_then(|v|v.parse().ok()).ok_or("Invalid phrase tick.")?;events.push((tick,format!("  {phrase}")));}
            events.sort_by_key(|e|e.0);
            chart.push_str(&format!("\n[{}{part}]\n{{\n{}\n}}\n",crate::engine::TIERS[tier],events.into_iter().map(|e|e.1).collect::<Vec<_>>().join("\n")));
        }
        counts.push((part.to_string(),part_counts));
    }
    if counts.is_empty(){return Err("No playable Expert instrument sections found in this .chart file.".into());}
    Ok(Generated{chart,counts})
}

#[cfg(test)] mod tests {
    use super::*;
    const CHART:&str="[Song]\n{\n Resolution = 192\n}\n[ExpertSingle]\n{\n 0 = N 4 300\n 48 = N 3 0\n 96 = N 2 0\n 192 = N 1 0\n 0 = S 2 384\n}\n";
    #[test] fn preserves_source_and_generates_progressive_densities(){let g=generate(CHART).unwrap();assert!(g.chart.starts_with(CHART));assert_eq!(g.counts[0].1,[2,3,4]);assert!(g.chart.contains("[EasySingle]"));assert!(g.chart.contains("0 = S 2 384"));assert!(generate(&g.chart).is_err());}
    #[test] fn rejects_bad_or_unsupported_input(){assert!(generate("bad").is_err());assert!(generate(&CHART.replace("ExpertSingle","ExpertKeys")).is_err());}
    #[test] fn ghl_black_three_is_a_note_and_modifiers_are_not(){let input=CHART.replace("ExpertSingle","ExpertGHLGuitar").replace("0 = N 4 300","0 = N 8 300\n 0 = N 6 0");let g=generate(&input).unwrap();assert!(g.chart.split("[HardGHLGuitar]").nth(1).unwrap().contains("0 = N 8"));assert!(!g.chart.split("[HardGHLGuitar]").nth(1).unwrap().contains("= N 6"));}
    #[test] fn drum_cymbals_follow_retained_hits_and_double_kick_becomes_kick(){let input=CHART.replace("ExpertSingle","ExpertDrums").replace("0 = N 4 300","0 = N 32 0\n 0 = N 2 0\n 0 = N 66 0");let g=generate(&input).unwrap();let easy=g.chart.split("[EasyDrums]").nth(1).unwrap().split("[MediumDrums]").next().unwrap();assert!(easy.contains("0 = N 0 0"));assert!(easy.contains("0 = N 66 0"));assert!(!easy.contains("= N 32"));}
}
