use crate::{audio,engine::{self,Options,Score,ROLES,TIERS}};
use eframe::egui::{self,Color32,RichText,Vec2,Stroke};
use std::{path::PathBuf,sync::mpsc::{self,Receiver},thread};
const BG:Color32=Color32::from_rgb(13,17,25);
const CARD:Color32=Color32::from_rgb(22,28,39);
const MUTED:Color32=Color32::from_rgb(141,155,177);
const ACCENT:Color32=Color32::from_rgb(200,120,255);
const LINE:Color32=Color32::from_rgb(43,54,72);
enum Job{StemSetup(Result<PathBuf,String>),Separated(Result<crate::stems::Separated,String>),Loaded(Result<(Score,PathBuf),String>),Exported(Result<PathBuf,String>)}
#[derive(serde::Serialize,serde::Deserialize)]
struct Project {version:u32,source:PathBuf,options:Options}
struct CollisionSnapshot {
    track: usize,
    chart_shift_bars: i32,
    mapping: std::collections::BTreeMap<i32, i32>,
    collisions: Vec<engine::DrumCollision>,
    change: Option<isize>,
}
#[derive(PartialEq)]
struct PatternKey {
    role:usize,tier:usize,bpm:u64,constant_bpm:bool,shift:i32,tracks:[Option<usize>;7],
    mapping:std::collections::BTreeMap<i32,i32>,dynamics:bool,double_bass:bool,open_bass:bool,strict:bool,ghl_open:bool,ghl_fold:bool,ghl_barres:bool,star_power:bool,short_sustains:bool,
}
struct PatternCache {key:PatternKey,notes:Result<std::sync::Arc<Vec<engine::Gem>>,String>,findings:Vec<crate::pitched::Finding>,star_power:crate::star_power::Plan,barres:std::collections::BTreeSet<(i64,usize)>,fidelity:Option<serde_json::Value>}
struct WaveDrag {origin_ms:i64,pixels:f32,resume:bool}
pub struct Studio {
    score:Option<Score>,source:Option<PathBuf>,options:Options,page:usize,
    role:usize,tier:usize,preview_start:f32,status:String,error:bool,
    job:Option<Receiver<Job>>,exported:Option<PathBuf>,help_search:String,capture_frames:u32,
    collision_snapshot: Option<CollisionSnapshot>,
    preview_audio:audio::PreviewAudio,chart_cursor:f64,volume:f32,
    wave_drag:Option<WaveDrag>,pattern_cache:Option<PatternCache>,stem_log:Option<PathBuf>,
}
impl Studio {
    pub fn new(cc:&eframe::CreationContext<'_>)->Self {
        let ctx=&cc.egui_ctx;let mut visuals=egui::Visuals::dark();
        visuals.panel_fill=BG;visuals.window_fill=CARD;visuals.extreme_bg_color=Color32::from_rgb(17,22,32);
        visuals.override_text_color=Some(Color32::from_rgb(230,237,247));
        visuals.selection.bg_fill=Color32::from_rgb(71,42,93);visuals.selection.stroke=Stroke::new(1.0_f32,ACCENT);
        visuals.widgets.inactive.bg_fill=Color32::from_rgb(31,40,55);visuals.widgets.inactive.weak_bg_fill=Color32::from_rgb(31,40,55);
        visuals.widgets.inactive.bg_stroke=Stroke::new(1.0_f32,LINE);visuals.widgets.hovered.bg_fill=Color32::from_rgb(47,63,79);
        visuals.widgets.active.bg_fill=Color32::from_rgb(75,43,100);ctx.set_visuals(visuals);
        let mut style=(*ctx.style()).clone();style.spacing.item_spacing=Vec2::new(12.,8.);
        style.spacing.button_padding=Vec2::new(14.,10.);style.spacing.interact_size.y=34.;
        style.text_styles.insert(egui::TextStyle::Body,egui::FontId::proportional(15.));
        style.text_styles.insert(egui::TextStyle::Button,egui::FontId::proportional(15.));
        ctx.set_style(style);
        let mut s=Self{score:None,source:None,options:Options::default(),page:0,role:1,tier:3,preview_start:0.,status:"Ready. Start with a Guitar Pro score or explore a demo.".into(),error:false,job:None,exported:None,help_search:String::new(),capture_frames:0,collision_snapshot:None,preview_audio:audio::PreviewAudio::default(),chart_cursor:0.,volume:0.6,wave_drag:None,pattern_cache:None,stem_log:None};
        if std::env::args().any(|a|a=="--demo"){s.load(engine::asset_root().join("examples/demo-fretted.gp"));}
        if std::env::args().any(|a|a=="--demo-ghl"){s.load(engine::asset_root().join("examples/demo-ghl.gp"));}
        if std::env::args().any(|a|a=="--demo-vocals"||a=="--demo-lyrics"){s.load(engine::asset_root().join("examples/demo-fretted.gp"));s.options.lyrics=Some(engine::asset_root().join("examples/demo-lyrics.lrc"));s.options.audio=Some(engine::asset_root().join("examples/demo-vocals.wav"));}
        if std::env::args().any(|a|a=="--demo-drums"){s.load(engine::asset_root().join("examples/demo.gp"));}
        if std::env::args().any(|a|a=="--demo-audio"){s.options.audio=Some(engine::asset_root().join("examples/demo-audio.wav"));}
        if let Some(page)=std::env::args().find_map(|a|a.strip_prefix("--page=").and_then(|p|p.parse::<usize>().ok())){s.page=page.min(5);}s
    }
    fn load(&mut self,path:PathBuf){let (tx,rx)=mpsc::channel();self.job=Some(rx);self.status="Reading score and expanding playback…".into();self.error=false;
        thread::spawn(move||{let result=engine::read_score(&path).map(|s|(s,path));let _=tx.send(Job::Loaded(result));});}
    fn accept(&mut self,score:Score,path:PathBuf){self.options.title=score.title.clone();self.options.artist=score.artist.clone();self.options.album=score.album.clone();self.options.bpm=score.tempos.first().map(|t|t.bpm).unwrap_or(120.);
        self.collision_snapshot=None;
        self.pattern_cache=None;self.wave_drag=None;self.preview_audio.player.stop();self.chart_cursor=0.;self.options.set_chart_quarters(0);
        if path==engine::asset_root().join("examples/demo-vocals.gp"){self.options.audio=Some(engine::asset_root().join("examples/demo-vocals.wav"));}
        self.options.gp_lyric_track=score.vocal_lyrics.first().map(|c|c.track);self.options.gp_lyric_verse=0;
        self.options.tracks=[None;7];self.options.tracks[0]=score.tracks.iter().find(|t|t.percussion).map(|t|t.index);
        let pitched:Vec<_>=score.tracks.iter().filter(|t|!t.percussion && !["vocal","voice"].iter().any(|word|t.name.to_lowercase().contains(word))).collect();
        self.options.tracks[1]=pitched.iter().find(|t|!t.name.to_lowercase().contains("bass")).or_else(||pitched.first()).map(|t|t.index);
        self.options.tracks[2]=pitched.iter().find(|t|t.name.to_lowercase().contains("bass")).map(|t|t.index);
        self.options.tracks[3]=pitched.iter().find(|t|t.name.to_lowercase().contains("rhythm")).map(|t|t.index);
        for t in &score.tracks{if t.percussion{for n in &t.notes{self.options.drum_map.entry(n.pitch).or_insert_with(||engine::default_drum(n.pitch));}}}
        if path==engine::asset_root().join("examples/demo-ghl.gp"){for role in 4..7{self.options.tracks[role]=self.options.tracks[role-3].or(self.options.tracks[1]);}}
        self.role=if path==engine::asset_root().join("examples/demo-ghl.gp"){4}else if self.options.tracks[1].is_some(){1}else{0};self.preview_start=0.;self.source=Some(path);self.exported=None;
        self.status=format!("Score loaded · {} tracks · {} notes",score.tracks.len(),score.tracks.iter().map(|t|t.notes.len()).sum::<usize>());self.score=Some(score);self.error=false;
    }
    fn poll(&mut self){let result=self.job.as_ref().and_then(|rx|rx.try_recv().ok());if let Some(job)=result{self.job=None;match job{
        Job::StemSetup(Ok(p))=>{self.options.stem_python=Some(p);self.status="Stem tools ready. Choose Separate song in Extra media.".into();self.error=false;},
        Job::Separated(Ok(s))=>{if self.options.recording_source().as_ref()==Some(&s.source){s.assign(&mut self.options);self.status="Separated vocals, guitar, bass and drums assigned. Export uses the remaining accompaniment as backing audio.".into();self.error=false;}else{self.status=format!("Recording changed while separating. Stems saved at {}; nothing assigned.",s.backing.parent().unwrap().display());self.error=true;}},
        Job::StemSetup(Err(e))|Job::Separated(Err(e))=>{self.status=e;self.error=true;},
        Job::Loaded(Ok((s,p)))=>self.accept(s,p),Job::Loaded(Err(e))|Job::Exported(Err(e))=>{self.status=e;self.error=true},
        Job::Exported(Ok(path))=>{let repairs=std::fs::read(path.join("conversion-report.json")).ok().and_then(|b|serde_json::from_slice::<serde_json::Value>(&b).ok()).and_then(|r|r["lyric_repairs"].as_array().map(|a|a.len())).unwrap_or(0);self.status=format!("Export complete · {}{}",path.display(),if repairs>0{format!(" · {repairs} repaired lyric times: see LYRIC-REVIEW.txt")}else{String::new()});self.exported=Some(path);self.error=false;}
    }}}
    fn start_export(&mut self){if let Some(score)=self.score.clone(){let o=self.options.clone();let(tx,rx)=mpsc::channel();self.job=Some(rx);self.status="Building and packaging your song…".into();self.error=false;thread::spawn(move||{let _=tx.send(Job::Exported(engine::export(&score,&o)));});}}
    fn save_project(&mut self){if let Some(source)=&self.source {if let Some(path)=rfd::FileDialog::new().add_filter("FretForge project",&["json"]).set_file_name("chart-project.json").save_file(){let project=Project{version:1,source:source.clone(),options:self.options.clone()};match serde_json::to_vec_pretty(&project).map_err(|e|e.to_string()).and_then(|b|std::fs::write(path,b).map_err(|e|e.to_string())){Ok(())=>{self.status="Project saved. Source files remain in their original locations.".into();self.error=false},Err(e)=>{self.status=e;self.error=true}}}}}
    fn open_project(&mut self){if let Some(path)=rfd::FileDialog::new().add_filter("FretForge project",&["json"]).pick_file(){let result=(||->Result<(Project,Score),String>{let p:Project=serde_json::from_slice(&std::fs::read(path).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;if p.version!=1{return Err("Unsupported project version.".into())}let score=engine::read_score(&p.source)?;Ok((p,score))})();match result{Ok((p,s))=>{self.accept(s,p.source);self.options=p.options;},Err(e)=>{self.status=e;self.error=true}}}}
    fn sidebar(&mut self,ctx:&egui::Context){egui::SidePanel::left("navigation").exact_width(218.).resizable(false).frame(egui::Frame::new().fill(Color32::from_rgb(17,22,32)).inner_margin(22.)).show(ctx,|ui|{
        ui.add_space(7.);let logo_id=egui::Id::new("fretforge-logo"); let logo=ui.ctx().data_mut(|d|d.get_temp::<egui::TextureHandle>(logo_id)).unwrap_or_else(||{let icon=crate::app_icon();let texture=ui.ctx().load_texture("FretForge",egui::ColorImage::from_rgba_unmultiplied([icon.width as usize,icon.height as usize],&icon.rgba),egui::TextureOptions::LINEAR);ui.ctx().data_mut(|d|d.insert_temp(logo_id,texture.clone()));texture});ui.image((logo.id(),Vec2::splat(64.)));ui.label(RichText::new("FretForge").size(18.).strong());ui.label(RichText::new("by ShyTheProgrammer - v1.6.0").size(10.).color(MUTED));ui.add_space(36.);
        for (i,num,name) in [(0,"01","Score & tracks"),(1,"02","Song details"),(2,"03","Extra media"),(5,"04","Lyrics"),(3,"05","Export"),(4,"?","Guide")]{let active=self.page==i;
            let text=RichText::new(format!("{num}   {name}")).color(if active{ACCENT}else{MUTED});if ui.add_sized([174.,43.],egui::Button::new(text).fill(if active{Color32::from_rgb(48,32,64)}else{Color32::TRANSPARENT}).stroke(Stroke::NONE)).clicked(){self.page=i}ui.add_space(4.);
        }
        ui.add_space(28.);ui.separator();ui.add_space(15.);ui.label(RichText::new("WORKSPACE").size(10.).color(MUTED));
        ui.add_enabled_ui(self.job.is_none(),|ui|{if ui.button("Open project").clicked(){self.open_project()}if ui.add_enabled(self.score.is_some(),egui::Button::new("Save project")).clicked(){self.save_project()}});
        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT),|ui|{ui.label(RichText::new("LOCAL FILES · OFFLINE").size(10.).color(ACCENT));ui.label(RichText::new("Your next chart starts here.").size(12.).color(MUTED));ui.add_space(12.);});
    });}
    fn header(&mut self,ui:&mut egui::Ui){ui.horizontal(|ui|{ui.vertical(|ui|{ui.label(RichText::new("YOUR CHARTING WORKSPACE").size(10.).color(ACCENT));ui.label(RichText::new(["From score to stage.","Make it your song.","Bring the song to life.","Ready for the next stage.","A little guidance.","Every word, on time."][self.page]).size(30.).strong());});ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{if self.job.is_some(){ui.spinner();ui.label(RichText::new("Working").color(MUTED));}else{ui.label(RichText::new("Offline studio").color(ACCENT).size(12.));}});});ui.add_space(10.);}
    fn source_page(&mut self,ui:&mut egui::Ui){
        card(ui,|ui|{ui.horizontal(|ui|{ui.vertical(|ui|{kicker(ui,"SOURCE SCORE");ui.label(RichText::new(self.source.as_ref().and_then(|p|p.file_name()).map(|n|n.to_string_lossy().into_owned()).unwrap_or("Drop a Guitar Pro file here".into())).size(21.).strong());ui.label(RichText::new("GP3, GP4, GP5, GPX, GP7 & GP8 · Up to 64 MB").color(MUTED).size(12.));});
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{ui.add_enabled_ui(self.job.is_none(),|ui|{if primary(ui,"Browse score"){if let Some(p)=rfd::FileDialog::new().add_filter("Guitar Pro",&["gp","gpx","gp3","gp4","gp5"]).pick_file(){self.load(p)}}});});});
            if self.score.is_none(){ui.add_space(14.);ui.horizontal(|ui|{ui.label(RichText::new("Just exploring?").color(MUTED));if ui.button("Try guitar demo").clicked(){self.load(engine::asset_root().join("examples/demo-fretted.gp"))}if ui.button("Try GHL demo").clicked(){self.load(engine::asset_root().join("examples/demo-ghl.gp"))}if ui.button("Try drum demo").clicked(){self.load(engine::asset_root().join("examples/demo.gp"))}if ui.button("Try lyric demo").clicked(){self.page=5;self.options.lyrics=Some(engine::asset_root().join("examples/demo-lyrics.lrc"));self.options.lyric_text=None;self.options.lyric_repairs.clear();self.options.lyric_offset=0;self.options.audio=Some(engine::asset_root().join("examples/demo-vocals.wav"));self.load(engine::asset_root().join("examples/demo-fretted.gp"))}});}
        });ui.add_space(16.);
        if let Some(score)=self.score.clone(){
            ui.horizontal_wrapped(|ui|{stat(ui,"TRACKS",&score.tracks.len().to_string());stat(ui,"SOURCE NOTES",&score.tracks.iter().map(|t|t.notes.len()).sum::<usize>().to_string());stat(ui,"RESOLUTION",&format!("{} PPQ",score.ppq));ui.vertical(|ui|{ui.set_min_width(145.);ui.label(RichText::new("OVERALL CHART BPM").size(10.).color(ACCENT));bpm_editor(ui,&mut self.options.bpm,&mut self.options.constant_bpm);});});ui.add_space(18.);
            card(ui,|ui|{kicker(ui,"BUILD YOUR LINEUP");ui.label(RichText::new("Choose the parts you want to play.").size(19.).strong());ui.add_space(8.);
                for (role,name) in ROLES.iter().enumerate(){ui.horizontal(|ui|{
                    let mut enabled=self.options.tracks[role].is_some();if ui.add_sized([180.,34.],egui::Checkbox::new(&mut enabled,*name)).on_hover_text(if role==0{"Include the selected percussion track as Pro Drums on enabled difficulties."}else if role>=4{"Include the selected Guitar Pro track as a six-fret GHL part. Musical button order is B1/B2/B3/W1/W2/W3."}else{"Include the selected Guitar Pro track as a five-fret part on enabled difficulties."}).changed(){self.options.tracks[role]=if enabled{if role>=4{self.options.tracks[role-3].or_else(||score.tracks.iter().find(|t|!t.percussion).map(|t|t.index))}else{score.tracks.iter().find(|t|t.percussion==(role==0)).map(|t|t.index)}}else{None};}
                    let selected=self.options.tracks[role].and_then(|i|score.tracks.iter().find(|t|t.index==i)).map(|t|t.name.clone()).unwrap_or("Not included".into());
                    egui::ComboBox::from_id_salt(("track",role)).selected_text(selected).width(250.).show_ui(ui,|ui|{ui.selectable_value(&mut self.options.tracks[role],None,"Not included");for track in &score.tracks{if track.percussion==(role==0){ui.selectable_value(&mut self.options.tracks[role],Some(track.index),format!("{} · {} notes",track.name,track.notes.len()));}}});
                    ui.label(RichText::new(if role==0{"PRO DRUMS"}else if role>=4{"6 FRET"}else{"5 FRET"}).size(10.).color(MUTED));
                });}
            });ui.add_space(16.);
            self.preview(ui,&score);
            let mapping_before=self.options.drum_map.clone();
            if self.options.tracks[0].is_some(){ui.add_space(14.);card(ui,|ui|{egui::CollapsingHeader::new("Drum mapping & dynamics").default_open(std::env::args().any(|a|a=="--capture")&&std::env::args().any(|a|a=="--demo-drums")).show(ui,|ui|{
                ui.horizontal_wrapped(|ui|{ui.checkbox(&mut self.options.strict,"Strict mapping").on_hover_text("Block export when two source drum hits occupy the same pad at the same tick. Ignored sounds are excluded.");ui.checkbox(&mut self.options.dynamics,"Infer velocity dynamics").on_hover_text("Infer Expert ghost and accent markers from source velocity when written dynamics are absent.");ui.checkbox(&mut self.options.double_bass,"Automatic 2× kick").on_hover_text("Alternate rapid Expert kick hits into the secondary kick lane above the automatic double-kick threshold.");});
                let mut pitches:BTreeMapCompat=std::collections::BTreeMap::new();if let Some(t)=score.tracks.iter().find(|t|Some(t.index)==self.options.tracks[0]){for n in &t.notes{*pitches.entry(n.pitch).or_insert(0)+=1;}}
                let collisions=engine::drum_collisions(&score,&self.options);let trouble=collisions.iter().flat_map(|c|c.pitches.iter().copied()).collect::<std::collections::BTreeSet<_>>();
                ui.label(RichText::new("Red = ignored · Yellow = same-pad overlapping hits").size(12.).color(MUTED));
                for (p,count) in pitches{let ignored=self.options.drum_map.get(&p).copied().unwrap_or(-1)<0;let color=if ignored{Color32::from_rgb(246,99,110)}else if trouble.contains(&p){Color32::from_rgb(245,207,92)}else{Color32::from_rgb(230,237,247)};ui.horizontal(|ui|{ui.add_sized([320.,34.],egui::Label::new(RichText::new(format!("{} · MIDI {p} · {count} {}{}",midi_drum_name(p),if count==1{"hit"}else{"hits"},if ignored{" · IGNORED"}else if trouble.contains(&p){" · COLLISION"}else{""})).color(color)).wrap());let lane=self.options.drum_map.entry(p).or_insert(-1);let text=lane_name(*lane);egui::ComboBox::from_id_salt(("drum",p)).selected_text(RichText::new(text).color(color)).width(230.).show_ui(ui,|ui|{for (code,label) in [(-1,"Ignore"),(0,"Kick"),(1,"Snare · red"),(2,"High tom · yellow"),(3,"Mid tom · blue"),(4,"Low tom · green"),(66,"Hi-hat · yellow cymbal"),(67,"Ride · blue cymbal"),(68,"Crash · green cymbal")]{ui.selectable_value(lane,code,label);}});});}
                ui.label(RichText::new("Strict export blocks same-lane collisions. Explicit Ignore entries are accepted.").size(12.).color(MUTED));
            });});}
            if self.options.drum_map!=mapping_before {ui.ctx().request_repaint();}
        }else{ui.add_space(22.);ui.label(RichText::new("A better starting point for your next chart.").size(24.).strong());ui.label(RichText::new("Import a score, choose your lineup, then shape the song before exporting.").color(MUTED));ui.add_space(20.);ui.columns(3,|cols|{for (i,(title,body)) in [("01  Import","Real playback timing, repeats and tuplets."),("02  Shape","Four difficulties, five- and six-fret parts and Pro Drums."),("03  Export","One song folder, ready for review in Moonscraper.")].iter().enumerate(){card(&mut cols[i],|ui|{ui.label(RichText::new(*title).color(ACCENT).size(18.));ui.label(RichText::new(*body).color(MUTED));});}});}
    }
    fn sync_collisions(&mut self,score:&Score) {
        let Some(track)=self.options.tracks[0] else {self.collision_snapshot=None;return;};
        if self.collision_snapshot.as_ref().is_some_and(|s|s.track==track&&s.mapping==self.options.drum_map&&s.chart_shift_bars==self.options.total_chart_quarters()){return;}
        let collisions=engine::drum_collisions(score,&self.options);
        let overlap_count=|items:&[engine::DrumCollision]|items.iter().map(|c|c.pitches.len()-1).sum::<usize>();
        let change=self.collision_snapshot.as_ref().filter(|s|s.track==track).and_then(|s|if s.mapping==self.options.drum_map{s.change}else{Some(overlap_count(&collisions) as isize-overlap_count(&s.collisions) as isize)});
        self.collision_snapshot=Some(CollisionSnapshot{track,chart_shift_bars:self.options.total_chart_quarters(),mapping:self.options.drum_map.clone(),collisions,change});
    }
    fn preview_notes(&mut self,score:&Score)->Result<std::sync::Arc<Vec<engine::Gem>>,String> {
        let key=PatternKey{role:self.role,tier:self.tier,bpm:self.options.bpm.to_bits(),constant_bpm:self.options.constant_bpm,shift:self.options.total_chart_quarters(),tracks:self.options.tracks,mapping:self.options.drum_map.clone(),dynamics:self.options.dynamics,double_bass:self.options.double_bass,open_bass:self.options.open_bass,strict:self.role!=0&&self.options.strict,ghl_open:self.options.ghl_open_pedals,ghl_fold:self.options.ghl_octave_folding,ghl_barres:self.options.ghl_simple_barres,star_power:self.options.star_power,short_sustains:self.options.short_sustains};
        if !self.pattern_cache.as_ref().is_some_and(|c|c.key==key) {
            let mut options=self.options.clone();if self.role==0{options.strict=false;}
            let notes=engine::gems(score,&options,self.role,self.tier).map(std::sync::Arc::new);
            let findings=if self.role>0{notes.as_ref().map(|n|if self.role>=4{crate::ghl::audit(score,&options,n,self.tier)}else{crate::pitched::audit(score,&options,n,self.tier)}).unwrap_or_default()}else{Vec::new()};
            let star_power=notes.as_ref().map(|n|crate::star_power::plan(score,&options,n,self.role)).unwrap_or_default();
            let barres=if self.role>=4{notes.as_ref().map(|n|crate::ghl::barre_pairs(n)).unwrap_or_default()}else{Default::default()};
            let fidelity=if self.role>0&&self.tier==3{notes.as_ref().ok().and_then(|n|score.tracks.iter().find(|t|Some(t.index)==options.tracks[self.role]).map(|t|crate::fidelity::report(score,&options,t,n,self.role>=4)))}else{None};self.pattern_cache=Some(PatternCache{key,notes,findings,star_power,barres,fidelity});
        }
        self.pattern_cache.as_ref().unwrap().notes.clone()
    }
    fn timeline_duration(&self,score:&Score)->f64 {
        let notes=crate::timing::Timeline::new(score,&self.options).seconds(engine::chart_end_tick(score,&self.options) as f64);
        let audio=self.preview_audio.clip.as_ref().map(|clip|(clip.duration()+self.options.audio_alignment_ms() as f64/1000.).max(0.)).unwrap_or(0.);
        notes.max(audio).max(0.01)
    }
    fn play_from_cursor(&mut self,score:&Score) {
        let Some(clip)=self.preview_audio.clip.clone() else {return;};
        let end=self.timeline_duration(score);if self.chart_cursor>=end{self.chart_cursor=0.;self.preview_start=0.;}
        match self.preview_audio.player.start(&clip,self.options.audio_alignment_ms(),self.chart_cursor,end,self.volume) {
            Ok(())=>{self.status="Playing the aligned chart and recording.".into();self.error=false;},
            Err(error)=>{self.status=error;self.error=true;}
        }
    }
    fn retime_playback(&mut self,score:&Score) {
        if self.preview_audio.player.playing(){self.chart_cursor=self.preview_audio.player.position().unwrap_or(self.chart_cursor);self.play_from_cursor(score);}else{self.preview_audio.player.stop();}
    }
    fn preview(&mut self,ui:&mut egui::Ui,score:&Score){self.sync_collisions(score);card(ui,|ui|{
        ui.horizontal(|ui|{kicker(ui,"PATTERN PREVIEW");ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{egui::ComboBox::from_id_salt("preview_tier").selected_text(TIERS[self.tier]).show_ui(ui,|ui|{for (i,t) in TIERS.iter().enumerate(){ui.selectable_value(&mut self.tier,i,*t);}});egui::ComboBox::from_id_salt("preview_role").selected_text(ROLES[self.role]).show_ui(ui,|ui|{for (i,r) in ROLES.iter().enumerate(){if self.options.tracks[i].is_some(){ui.selectable_value(&mut self.role,i,*r);}}});});});
        self.video_picker(ui);let audio_before=self.options.audio.clone();file_picker(ui,"Preview recording",&mut self.options.audio,&["ogg","opus","mp3","wav"]);
        if audio_before!=self.options.audio{self.preview_audio.sync(self.options.recording_source());self.wave_drag=None;ui.ctx().request_repaint();}
        if self.preview_audio.loading(){ui.horizontal(|ui|{ui.spinner();ui.label(RichText::new("Reading the recording and preparing its waveform…").color(MUTED));});}
        if let Some(error)=&self.preview_audio.error{ui.label(RichText::new(error).color(Color32::from_rgb(247,139,126)));}
        ui.horizontal_wrapped(|ui|{
            let ready=self.preview_audio.clip.is_some();let playing=self.preview_audio.player.playing();
            if ui.add_enabled(ready,egui::Button::new(if playing{"Pause"}else{"Play"}).fill(if playing{LINE}else{Color32::from_rgb(71,42,93)})).clicked(){if playing{self.preview_audio.player.pause();}else{self.play_from_cursor(score);}ui.ctx().request_repaint();}
            if ui.button("Stop").clicked(){self.preview_audio.player.stop();self.chart_cursor=0.;self.preview_start=0.;ui.ctx().request_repaint();}
            ui.label(format!("{:.2}s / {:.2}s",self.chart_cursor,self.timeline_duration(score)));
            if ui.add(egui::Slider::new(&mut self.volume,0.0..=1.0).text("Volume").show_value(false)).changed(){self.preview_audio.player.volume(self.volume);}
        });
        ui.horizontal_wrapped(|ui|{
            ui.label(RichText::new("All charted notes").color(MUTED));
            let earliest=engine::minimum_chart_shift_quarters(score,&self.options);
            for (label,delta) in [("< 1 bar",-4),("< 1/4 bar",-1),("1/4 bar >",1),("1 bar >",4)]{
                let next=self.options.total_chart_quarters()+delta;
                if ui.add_enabled(next>=earliest&&next<=40000,egui::Button::new(label)).on_hover_text("Move every instrument and difficulty together.").clicked(){self.options.set_chart_quarters(next);self.retime_playback(score);ui.ctx().request_repaint();}
            }
            ui.label(RichText::new(format!("{:+.2} bars",self.options.total_chart_quarters() as f64/4.)).color(ACCENT));
            if ui.button("Reset notes").clicked(){self.options.set_chart_quarters(0);self.retime_playback(score);}

        });
        ui.horizontal_wrapped(|ui|{
            let mut alignment=self.options.audio_alignment_ms();ui.label(RichText::new("Audio alignment").color(MUTED));
            if ui.add_enabled(self.options.recording_source().is_some(),egui::DragValue::new(&mut alignment).range(-3_600_000..=3_600_000).speed(10).suffix(" ms")).changed(){self.options.set_audio_alignment_ms(alignment);self.retime_playback(score);ui.ctx().request_repaint();}
            if ui.add_enabled(alignment!=0,egui::Button::new("Reset audio")).clicked(){self.options.set_audio_alignment_ms(0);self.retime_playback(score);ui.ctx().request_repaint();}
        });
        self.sync_collisions(score);
        let total=self.timeline_duration(score);let timeline=crate::timing::Timeline::new(score,&self.options);
        let window=self.options.pattern_preview_beats.clamp(1.,64.);
        let max=(timeline.tick(total)/score.ppq as f64-window as f64).max(0.) as f32;
        self.preview_start=self.preview_start.clamp(0.,max);
        let cursor_beat=timeline.tick(self.chart_cursor)/score.ppq as f64;
        if self.preview_audio.player.playing()&&(cursor_beat<self.preview_start as f64||cursor_beat>=self.preview_start as f64+window as f64){self.preview_start=((cursor_beat/window as f64).floor()*window as f64).min(max as f64) as f32;}
        // Cached mapping stays available even when strict export rejects collisions.
        let notes=self.preview_notes(score);
        let (rect,response)=ui.allocate_exact_size(Vec2::new(ui.available_width(),if self.role>=4{225.}else{180.}),egui::Sense::click_and_drag());let painter=ui.painter_at(rect);painter.rect_filled(rect,8.,Color32::from_rgb(15,20,29));
        let colors=if self.role>=4{[Color32::from_gray(100),Color32::from_gray(100),Color32::from_gray(100),Color32::from_gray(230),Color32::from_gray(230),Color32::from_gray(230),ACCENT]}else{[Color32::from_rgb(91,220,146),Color32::from_rgb(242,100,124),Color32::from_rgb(245,207,92),Color32::from_rgb(91,162,247),Color32::from_rgb(245,158,76),ACCENT,ACCENT]};
        let left=rect.left()+if self.role>=4{48.}else{22.};let width=rect.right()-22.-left;let start=(self.preview_start*score.ppq as f32).round() as i64;let span=(score.ppq as f32*window).round() as i64;
        let start_seconds=timeline.seconds(start as f64);let span_seconds=timeline.seconds((start+span) as f64)-start_seconds;
        if self.preview_audio.clip.is_some()&&response.drag_started(){let resume=self.preview_audio.player.playing();self.chart_cursor=self.preview_audio.player.position().unwrap_or(self.chart_cursor);self.preview_audio.player.stop();self.wave_drag=Some(WaveDrag{origin_ms:self.options.audio_alignment_ms(),pixels:0.,resume});}
        if response.dragged(){if let Some(drag)=&mut self.wave_drag{drag.pixels+=response.drag_delta().x;let delta=(drag.pixels as f64/width as f64*span_seconds*1000.).round() as i64;self.options.set_audio_alignment_ms(drag.origin_ms+delta);ui.ctx().request_repaint();}}
        if response.drag_stopped(){if let Some(drag)=self.wave_drag.take(){if drag.resume{self.play_from_cursor(score);}ui.ctx().request_repaint();}}
        if response.clicked(){if let Some(position)=response.interact_pointer_pos(){let at=((position.x-left)/width).clamp(0.,1.) as f64;let was_playing=self.preview_audio.player.playing();self.preview_audio.player.stop();self.chart_cursor=timeline.seconds(start as f64+at*span as f64).min(total);if was_playing{self.play_from_cursor(score);}ui.ctx().request_repaint();}}
        if let Some(clip)=&self.preview_audio.clip {
            let shift=self.options.audio_alignment_ms() as f64/1000.;let bins=width.ceil().max(1.) as usize;
            for pixel in 0..bins{let from=timeline.seconds(start as f64+pixel as f64/bins as f64*span as f64)-shift;let to=timeline.seconds(start as f64+(pixel+1) as f64/bins as f64*span as f64)-shift;let (low,high)=clip.envelope(from,to);let x=left+pixel as f32;let center=rect.center().y;let scale=rect.height()*0.45;painter.line_segment([egui::pos2(x,center-high*scale),egui::pos2(x,center-low*scale)],Stroke::new(1.0_f32,Color32::from_rgba_unmultiplied(108,231,198,65)));}
            let origin_x=left+((timeline.tick(shift)-start as f64)/span as f64) as f32*width;
            if origin_x>=left&&origin_x<=rect.right()-22.{painter.line_segment([egui::pos2(origin_x,rect.top()+3.),egui::pos2(origin_x,rect.bottom()-3.)],Stroke::new(1.0_f32,ACCENT));}
        }
        if let Some(cache)=&self.pattern_cache{for phrase in &cache.star_power.phrases{let end=phrase.start+phrase.length;if end>start&&phrase.start<start+span{let x1=left+width*(phrase.start-start).max(0) as f32/span as f32;let x2=left+width*(end-start).min(span) as f32/span as f32;let band=egui::Rect::from_min_max(egui::pos2(x1,rect.top()+3.),egui::pos2(x2,rect.top()+9.));painter.rect_filled(band,2.,Color32::from_rgb(245,207,92));}}}
        for lane in 0..if self.role>=4{7}else{5}{let y=rect.top()+32.+lane as f32*27.;if self.role>=4{painter.text(egui::pos2(rect.left()+5.,y),egui::Align2::LEFT_CENTER,["B1","B2","B3","W1","W2","W3","OPEN"][lane],egui::FontId::proportional(10.),colors[lane]);}painter.line_segment([egui::pos2(left,y),egui::pos2(rect.right()-22.,y)],Stroke::new(1.0_f32,LINE));if self.role<4{painter.circle_filled(egui::pos2(rect.left()+9.,y),3.,colors[lane]);}}
        for beat in 0..=window.ceil() as i32{let x=left+width*beat as f32/window;painter.line_segment([egui::pos2(x,rect.top()+10.),egui::pos2(x,rect.bottom()-10.)],Stroke::new(if beat%4==0{1.5_f32}else{0.5_f32},LINE));}
        match notes{Ok(notes)=>{for n in notes.iter().filter(|g|g.tick>=start && g.tick<start+span){let lane=if self.role>=4{let Some(lane)=crate::ghl::preview_lane(n.lane)else{continue};lane as i32}else{if n.lane==32||n.lane==7{0}else{n.lane}};if !(0..if self.role>=4{7}else{5}).contains(&lane){continue}let x=left+width*(n.tick-start) as f32/span as f32;let y=rect.top()+32.+lane as f32*27.;let c=colors[lane as usize];if n.length>0{let end=(x+width*n.length as f32/span as f32).min(rect.right()-22.);painter.line_segment([egui::pos2(x,y),egui::pos2(end,y)],Stroke::new(4.0_f32,c.gamma_multiply(0.45)));}if self.role>=4&&self.pattern_cache.as_ref().is_some_and(|cache|cache.barres.contains(&(n.tick,lane as usize%3))){painter.rect_filled(egui::Rect::from_center_size(egui::pos2(x,y),Vec2::splat(12.)),1.,c);}else{painter.circle_filled(egui::pos2(x,y),6.,c);painter.circle_stroke(egui::pos2(x,y),8.,Stroke::new(1.0_f32,c.gamma_multiply(0.4)));}}ui.label(RichText::new(format!("{} playable events · {}-beat window",notes.iter().filter(|g|engine::playable(g.lane)).count(),window)).color(MUTED).size(12.));},Err(e)=>{ui.label(RichText::new(e).color(Color32::from_rgb(246,164,119)));}}
        if let Some(cache)=&self.pattern_cache{if self.options.star_power{ui.label(RichText::new(format!("Star Power: {} phrase{} · gold bands · four-measure minimum gap",cache.star_power.phrases.len(),if cache.star_power.phrases.len()==1{""}else{"s"})).size(12.).color(Color32::from_rgb(245,207,92)));for warning in &cache.star_power.warnings{ui.label(RichText::new(warning).size(12.).color(Color32::from_rgb(246,164,119)));}}}
        if self.role==0 {if let Some(snapshot)=&self.collision_snapshot {for c in snapshot.collisions.iter().filter(|c|c.tick>=start&&c.tick<start+span&&(0..5).contains(&c.lane)){let x=left+width*(c.tick-start) as f32/span as f32;let y=rect.top()+32.+c.lane as f32*27.;painter.circle_stroke(egui::pos2(x,y),11.,Stroke::new(2.0_f32,Color32::from_rgb(255,178,105)));}}}
        if self.chart_cursor>=start_seconds&&self.chart_cursor<=start_seconds+span_seconds{let x=left+((timeline.tick(self.chart_cursor)-start as f64)/span as f64) as f32*width;painter.line_segment([egui::pos2(x,rect.top()+2.),egui::pos2(x,rect.bottom()-2.)],Stroke::new(2.0_f32,Color32::WHITE));}
        let was_playing=self.preview_audio.player.playing();
        if wide_scrubber(ui,&mut self.chart_cursor,0.0..=total,"Playhead (s)",2,None).changed(){self.preview_audio.player.stop();let beat=timeline.tick(self.chart_cursor)/score.ppq as f64;self.preview_start=((beat/4.).floor()*4.).min(max as f64) as f32;if was_playing{self.play_from_cursor(score);}ui.ctx().request_repaint();}
        let was_playing=self.preview_audio.player.playing();
        if wide_scrubber(ui,&mut self.preview_start,0.0..=max,"Start beat",2,Some(0.25)).changed(){self.preview_audio.player.stop();self.chart_cursor=timeline.seconds(self.preview_start as f64*score.ppq as f64);if was_playing{self.play_from_cursor(score);}ui.ctx().request_repaint();}
        ui.horizontal_wrapped(|ui|{
            for (label,delta) in [("< 1/4 beat",-0.25),("1/4 beat >",0.25)]{if ui.button(label).clicked(){self.preview_start=(self.preview_start+delta).clamp(0.,max);self.preview_audio.player.stop();self.chart_cursor=timeline.seconds(self.preview_start as f64*score.ppq as f64);if was_playing{self.play_from_cursor(score);}}}
            preview_zoom(ui,&mut self.options.pattern_preview_beats);
        });
        ui.label(RichText::new(if self.preview_audio.clip.is_some(){"Drag the waveform left/right to align the recording; click the preview to seek."}else{"Attach a recording to show its waveform and enable playback."}).size(12.).color(MUTED));
        let shift=self.options.audio_alignment_ms();
        if self.options.audio.is_some(){ui.label(RichText::new(if shift>0{format!("Export adds {shift} ms of leading silence to the recording and full-length stems.")}else if shift<0{format!("Export trims {} ms from the beginning of the recording and full-length stems.",-shift)}else{"Audio is aligned without padding or trimming.".into()}).size(12.).color(ACCENT));}
        if let Some(snapshot)=&self.collision_snapshot {
            ui.separator();kicker(ui,"LIVE DRUM COLLISIONS");
            let count=snapshot.collisions.len();let overlaps=snapshot.collisions.iter().map(|c|c.pitches.len()-1).sum::<usize>();
            let visible=snapshot.collisions.iter().filter(|c|c.tick>=start&&c.tick<start+span).count();
            let warning=Color32::from_rgb(255,178,105);
            ui.label(RichText::new(if count==0{"No drum collisions. Your current mapping is clear.".into()}else{format!("{count} collision {} · {overlaps} overlapping {} · {visible} in this preview window",if count==1{"location"}else{"locations"},if overlaps==1{"hit"}else{"hits"})}).color(if count==0{ACCENT}else{warning}).strong());
            if let Some(change)=snapshot.change {
                let (text,color)=if change<0{(format!("Improved: {} fewer overlapping hits since your last reassignment.",-change),ACCENT)}else if change>0{(format!("Increased: {change} more overlapping hits since your last reassignment."),warning)}else{("Unchanged: your last reassignment did not change the overlap count.".into(),MUTED)};
                ui.label(RichText::new(text).size(12.).color(color));
            }
            ui.label(RichText::new("Checks the whole selected drum track at Expert, including hits outside this window. Ignored pitches are excluded.").size(12.).color(MUTED));
            if count>0 {
                ui.label(RichText::new(if self.options.strict{"Preview merges colliding hits; strict export remains blocked until they are resolved."}else{"Preview and export merge colliding hits, losing the overlapping hits."}).size(12.).color(warning));
                egui::CollapsingHeader::new("Show collision details").show(ui,|ui|{
                    for c in snapshot.collisions.iter().take(12) {
                        let beat=c.tick as f64/score.ppq as f64;let seconds=timeline.seconds(c.tick as f64);
                        let names=c.pitches.iter().map(|p|format!("{} (MIDI {p})",midi_drum_name(*p))).collect::<Vec<_>>().join(" + ");
                        ui.horizontal_wrapped(|ui|{if ui.small_button(format!("Bar {} · {:.2}s",c.tick/(score.ppq*4)+1,seconds)).on_hover_text("Jump the preview to this collision").clicked(){self.preview_start=((beat as f32/4.).floor()*4.).min(max);self.preview_audio.player.stop();self.chart_cursor=timeline.seconds(self.preview_start as f64*score.ppq as f64);ui.ctx().request_repaint();}ui.label(format!("{}: {names}",lane_name(c.lane)));});
                    }
                    if count>12 {ui.label(RichText::new(format!("Showing the first 12 of {count} collision locations.")).size(12.).color(MUTED));}
                });
            }
        }
        if self.role>0{ui.separator();kicker(ui,if self.role>=4{"GHL SHAPES & PATTERNS"}else{"GUITAR / RHYTHM / BASS REVIEW"});
            if let Some(cache)=&self.pattern_cache{if let Some(f)=&cache.fidelity{ui.label(RichText::new(format!("Literal Expert · {} / {} source attacks · {} mapping/technique decisions",f["target_attack_events"],f["source_attack_events"],f["decisions"].as_array().unwrap().len())).color(MUTED));ui.collapsing("Source fidelity decisions",|ui|{for d in f["decisions"].as_array().unwrap().iter().take(100){ui.label(format!("Tick {} · {}: {}",d["tick"],d["kind"].as_str().unwrap(),d["detail"].as_str().unwrap()));}ui.label("Full source and target details are included in the export report. Review uncertain techniques against the audio.");});}let warnings=cache.findings.iter().filter(|f|f.severity=="warning").count();let count=cache.findings.len();let findings=cache.findings.iter().take(50).cloned().collect::<Vec<_>>();ui.label(RichText::new(format!("{} · {warnings} review warnings · {} {}",if self.role>=4{"Literal 6-fret v1.0"}else{"Literal 5-fret v1.0"},count-warnings,if self.role>=4{"notices"}else{"dense Expert notices"})).color(if warnings==0{ACCENT}else{Color32::from_rgb(255,178,105)}));
                ui.label(RichText::new(if self.role>=4{"Pattern notices identify common six-fret sequences. Squares mark simple barres. Full findings are in CHARTING-REVIEW.txt."}else{"Spacing targets prompt review; Expert attacks are kept. Full findings for all exported parts are in CHARTING-REVIEW.txt."}).size(12.).color(MUTED));
                egui::CollapsingHeader::new("Review this instrument and difficulty").default_open(std::env::args().any(|a|a=="--review-preview")).show(ui,|ui|{for finding in findings.iter().take(50){ui.horizontal_wrapped(|ui|{if ui.small_button(format!("{}:{:.2}",finding.measure,finding.beat)).on_hover_text("Jump to this passage").clicked(){self.preview_start=((finding.tick as f32/score.ppq as f32/4.).floor()*4.).clamp(0.,max);self.preview_audio.player.stop();self.chart_cursor=timeline.seconds(finding.tick as f64);ui.ctx().request_repaint();}ui.label(format!("{} · {:.1} ms · {:.1} BPM",finding.pattern,finding.delta_ms,finding.bpm));});ui.label(RichText::new(finding.suggested_fix).size(12.).color(MUTED));}if count>50{ui.label("Showing the first 50 findings. Export includes the complete review.");}});
            }
        }
    });}
    fn details(&mut self,ui:&mut egui::Ui){card(ui,|ui|{kicker(ui,"SONG IDENTITY");field(ui,"Title",&mut self.options.title);field(ui,"Artist",&mut self.options.artist);field(ui,"Album",&mut self.options.album);field(ui,"Genre",&mut self.options.genre);field(ui,"Year",&mut self.options.year);field(ui,"Charter",&mut self.options.charter);field(ui,"Loading phrase",&mut self.options.loading_phrase);field(ui,"Song icon name",&mut self.options.icon);});ui.add_space(16.);
        card(ui,|ui|{kicker(ui,"TIMING & DIFFICULTY");ui.horizontal_wrapped(|ui|{ui.label("Overall chart BPM");bpm_editor(ui,&mut self.options.bpm,&mut self.options.constant_bpm);ui.label("Audio alignment");let mut alignment=self.options.audio_alignment_ms();if ui.add(egui::DragValue::new(&mut alignment).range(-3_600_000..=3_600_000).suffix(" ms")).changed(){self.options.set_audio_alignment_ms(alignment);self.preview_audio.player.stop();}});ui.label(RichText::new("Use Pattern preview to play, align the waveform, and shift every charted part by a bar. Check Constant BPM to replace all source tempo changes with one starting tempo. Unchecked, Overall BPM scales the source tempo map. Time signatures are preserved.").color(MUTED).size(12.));
            ui.horizontal_wrapped(|ui|{ui.checkbox(&mut self.options.ghl_open_pedals,"GHL open pedal tones").on_hover_text("Use open notes for qualifying repeated low picked pedal tones. Turn off to require fretted notes.");ui.checkbox(&mut self.options.ghl_simple_barres,"GHL simple barres (Hard/Expert)").on_hover_text("Optional two-button same-column grips for deliberate, uncrowded shapes. Default off. No barre-plus-fret or four-to-six-button clusters. Square grips are flagged for controller play-testing.");ui.checkbox(&mut self.options.ghl_octave_folding,"GHL octave folding").on_hover_text("Collapse octave-equivalent melody pitches. Optional octave equivalence changes contour. Long phrases already use documented position resets without dropping attacks.");});
            ui.add_space(8.);ui.checkbox(&mut self.options.short_sustains,"Keep short notation sustains").on_hover_text("Off: ordinary quarter-note and shorter notation durations become short attacks; longer notes and explicit let-ring/bends retain tails. On: keep short Guitar Pro playback durations as sustains too. Applies to five- and six-fret Guitar, Rhythm and Bass.");ui.horizontal_wrapped(|ui|{for (i,name) in TIERS.iter().enumerate(){ui.checkbox(&mut self.options.difficulties[i],*name).on_hover_text(match i{0=>"Export Easy with sparse rhythmic anchors and simple single notes.",1=>"Export Medium with reduced density and simpler chords.",2=>"Export Hard with more source detail and fewer reductions.",_=>"Export Expert with every representable source attack and explicit source techniques."});} });ui.horizontal(|ui|{ui.checkbox(&mut self.options.star_power,"Generate Star Power").on_hover_text("Normally two measures, scaled to one–four by density. At least four measures between phrase end and the next start; section anchors and strong endings are preferred. Gold bands show phrases in the preview.");ui.checkbox(&mut self.options.open_bass,"Explicit Expert open bass").on_hover_text("Only explicit gameplay annotations can create purple Expert notes. Open strings in Guitar Pro stay colored notes; lower tiers never use opens.");});
            ui.add_space(10.);ui.label(RichText::new("Intensity ratings").size(17.).strong());ui.label(RichText::new("Auto ratings use note density on a 0–6 scale. Select a value to override.").color(MUTED).size(12.));
            if let Some(score)=&self.score{for (role,name) in ROLES.iter().enumerate(){if self.options.tracks[role].is_some(){let estimated=engine::gems(score,&self.options,role,3).map(|g|engine::rating(&g,score,&self.options)).ok();ui.horizontal(|ui|{ui.label(*name);rating_combo(ui,("rating",role),&mut self.options.ratings[role]);ui.label(RichText::new(estimated.map(|r|format!("Estimated {r} / 6")).unwrap_or("Adjust mapping to calculate".into())).color(MUTED));});}}}
            ui.horizontal(|ui|{ui.label("Overall song");rating_combo(ui,"overall",&mut self.options.overall);});
        });}
    fn lyric_duration(&self)->f64{let audio=self.preview_audio.clip.as_ref().map(|c|(c.duration()+self.options.audio_alignment_ms() as f64/1000.).max(0.)).unwrap_or(0.);let lyric=crate::lyrics::load(&self.options).map(|d|(d.entries.last().unwrap().ms+self.options.lyric_offset as i64+self.options.audio_alignment_ms()).max(0) as f64/1000.+2.).unwrap_or(0.);let chart=self.score.as_ref().map(|s|self.timeline_duration(s)).unwrap_or(0.);audio.max(lyric).max(chart).max(1.)}
    fn play_lyrics(&mut self){let Some(clip)=self.preview_audio.clip.clone()else{return;};let end=self.lyric_duration();if self.chart_cursor>=end{self.chart_cursor=0.;}match self.preview_audio.player.start(&clip,self.options.audio_alignment_ms(),self.chart_cursor,end,self.volume){Ok(())=>{self.status="Playing the recording with aligned lyrics.".into();self.error=false;},Err(e)=>{self.status=e;self.error=true;}}}
    fn lyrics_page(&mut self,ui:&mut egui::Ui){
        card(ui,|ui|{kicker(ui,"LRC FILES · CLONE HERO");let before=self.options.lyrics.clone();file_picker(ui,"LRC file",&mut self.options.lyrics,&["lrc"]);if before!=self.options.lyrics{self.options.lyric_text=None;self.options.lyric_repairs.clear();self.options.lyric_offset=0;}
            ui.horizontal_wrapped(|ui|{if ui.button("New lyrics").clicked(){self.options.lyrics=None;self.options.lyric_text=Some("[00:00.000]Your first lyric\n".into());self.options.lyric_repairs.clear();self.options.lyric_offset=0;}if ui.button("Try lyric demo").clicked(){self.options.lyrics=Some(engine::asset_root().join("examples/demo-lyrics.lrc"));self.options.lyric_text=None;self.options.lyric_repairs.clear();self.options.audio=Some(engine::asset_root().join("examples/demo-vocals.wav"));self.options.lyric_offset=0;}
                if ui.button("Save aligned LRC copy").clicked(){match crate::lyrics::load(&self.options){Ok(doc)=>{if let Some(path)=rfd::FileDialog::new().add_filter("LRC lyrics",&["lrc"]).set_file_name("lyrics-aligned.lrc").save_file(){match std::fs::write(&path,doc.lrc(self.options.lyric_offset as i64+self.options.audio_alignment_ms())){Ok(())=>{self.status=format!("Aligned lyrics saved · {}",path.display());self.error=false;},Err(e)=>{self.status=e.to_string();self.error=true;}}}},Err(e)=>{self.status=e;self.error=true;}}}
            });
            ui.label(RichText::new("Open standard or enhanced LRC, edit its words and times, and save a separate aligned copy. The original file is preserved. The copy includes lyric alignment and the recording's export alignment.").color(MUTED).size(12.));
            if let Some(score)=&self.score{if !score.vocal_lyrics.is_empty(){egui::CollapsingHeader::new("Make an LRC from Guitar Pro lyrics").show(ui,|ui|{ui.horizontal_wrapped(|ui|{egui::ComboBox::from_id_salt("gp-lyrics-track").selected_text(self.options.gp_lyric_track.and_then(|i|score.tracks.iter().find(|t|t.index==i)).map(|t|t.name.as_str()).unwrap_or("Choose lyric track")).show_ui(ui,|ui|{for t in &score.tracks{if score.vocal_lyrics.iter().any(|c|c.track==t.index){ui.selectable_value(&mut self.options.gp_lyric_track,Some(t.index),&t.name);}}});ui.label("Verse");let mut verse=self.options.gp_lyric_verse+1;if ui.add(egui::DragValue::new(&mut verse).range(1..=10)).changed(){self.options.gp_lyric_verse=verse-1;}if ui.button("Import as LRC").clicked(){if let Some(track)=self.options.gp_lyric_track{match crate::lyrics::from_guitar_pro(score,&self.options,track,self.options.gp_lyric_verse){Ok(text)=>{self.options.lyric_text=Some(text);self.options.lyrics=None;self.options.lyric_repairs.clear();self.options.lyric_offset=0;self.status="Guitar Pro lyric timing copied into the LRC editor.".into();self.error=false;},Err(e)=>{self.status=e;self.error=true;}}}}});ui.label(RichText::new("This copies the current score timing into editable LRC. Reimport after changing score tempo or bar position.").color(MUTED).size(12.));});}}
        });ui.add_space(16.);
        let document=crate::lyrics::load(&self.options);
        if let Ok(doc)=&document{self.options.lyric_repairs=doc.review(&self.options.lyric_repairs);}
        card(ui,|ui|{kicker(ui,"STARTING POINT & PLAYBACK");if let Some(video)=&self.options.music_video{ui.label(RichText::new(format!("Recording extracted from music video: {}",video.file_name().unwrap_or_default().to_string_lossy())).color(ACCENT));}file_picker(ui,"Recording",&mut self.options.audio,&["ogg","opus","mp3","wav"]);
            ui.horizontal_wrapped(|ui|{if ui.add_enabled(self.preview_audio.clip.is_some(),egui::Button::new(if self.preview_audio.player.playing(){"Pause"}else{"Play"})).clicked(){if self.preview_audio.player.playing(){self.preview_audio.player.pause();}else{self.play_lyrics();}}if ui.button("Stop").clicked(){self.preview_audio.player.stop();self.chart_cursor=0.;}ui.label(format!("Playhead {:.3}s",self.chart_cursor));});
            if let Some(error)=&self.preview_audio.error{ui.label(RichText::new(error).color(Color32::from_rgb(246,164,119)));}
            ui.horizontal_wrapped(|ui|{ui.label("Lyrics earlier / later");ui.add(egui::DragValue::new(&mut self.options.lyric_offset).range(-3_600_000..=3_600_000).speed(10).suffix(" ms"));if ui.button("-100 ms").clicked(){self.options.lyric_offset=(self.options.lyric_offset-100).max(-3_600_000);}if ui.button("+100 ms").clicked(){self.options.lyric_offset=(self.options.lyric_offset+100).min(3_600_000);}if ui.button("Reset alignment").clicked(){self.options.lyric_offset=0;}});
            if let Ok(doc)=&document{let first=doc.entries[0].ms;let total_shift=self.options.lyric_offset as i64+self.options.audio_alignment_ms();let mut target=(first+total_shift).max(0) as f64/1000.;ui.horizontal_wrapped(|ui|{ui.label("First lyric at");if ui.add(egui::DragValue::new(&mut target).range(0.0..=86400.0).speed(0.01).fixed_decimals(3).suffix(" s")).changed(){let target=crate::lyrics::snap_seconds(target,self.score.as_ref(),&self.options);self.options.lyric_offset=((target*1000.).round() as i64-first-self.options.audio_alignment_ms()).clamp(-3_600_000,3_600_000) as i32;}if ui.button("Align first lyric to playhead").clicked(){let at=crate::lyrics::snap_seconds(self.chart_cursor,self.score.as_ref(),&self.options);self.options.lyric_offset=((at*1000.).round() as i64-first-self.options.audio_alignment_ms()).clamp(-3_600_000,3_600_000) as i32;}});}
            ui.label(RichText::new("All lyric timestamps move together. Seek to the first sung word, then align the first lyric to the playhead. The recording and charted notes keep their positions.").color(MUTED).size(12.));
            ui.horizontal_wrapped(|ui|{ui.checkbox(&mut self.options.lyric_snap,"Snap lyrics to beats").on_hover_text("Snap lyric edits, row alignment and clicks in this viewer to the selected musical subdivision, using the current BPM and tempo changes. Free scrubbing remains available when off.");egui::ComboBox::from_id_salt("lyric-grid").selected_text(grid_name(self.options.lyric_snap_division)).show_ui(ui,|ui|{for division in [1,2,4,8,16]{ui.selectable_value(&mut self.options.lyric_snap_division,division,grid_name(division));}});});
            let end=self.lyric_duration();let playing=self.preview_audio.player.playing();if wide_scrubber(ui,&mut self.chart_cursor,0.0..=end,"Seek (s)",3,None).changed(){self.chart_cursor=crate::lyrics::snap_seconds(self.chart_cursor,self.score.as_ref(),&self.options).min(end);self.preview_audio.player.stop();if playing{self.play_lyrics();}}
            let timeline=self.score.as_ref().map(|score|crate::timing::Timeline::new(score,&self.options));
            let ppq=self.score.as_ref().map(|score|score.ppq as f64).unwrap_or(1.);let bpm=self.options.bpm.max(1.);
            let to_beat=|seconds:f64|timeline.as_ref().map(|t|t.tick(seconds)/ppq).unwrap_or(seconds*bpm/60.);
            let to_seconds=|beat:f64|timeline.as_ref().map(|t|t.seconds(beat*ppq)).unwrap_or(beat*60./bpm);
            ui.horizontal_wrapped(|ui|{for (label,delta) in [("< 1/4 beat",-0.25),("1/4 beat >",0.25)]{if ui.button(label).clicked(){self.chart_cursor=to_seconds((to_beat(self.chart_cursor)+delta).max(0.)).min(end);self.preview_audio.player.stop();if playing{self.play_lyrics();}}}preview_zoom(ui,&mut self.options.lyric_preview_beats);});
            let window=self.options.lyric_preview_beats.clamp(1.,64.) as f64;let start=(to_beat(self.chart_cursor)/window).floor()*window;let (r,response)=ui.allocate_exact_size(Vec2::new(ui.available_width(),100.),egui::Sense::click());let painter=ui.painter_at(r);painter.rect_filled(r,8.,BG);
            if let Some(clip)=&self.preview_audio.clip{let shift=self.options.audio_alignment_ms() as f64/1000.;let bins=r.width().ceil().max(1.) as usize;for pixel in 0..bins{let (low,high)=clip.envelope(to_seconds(start+pixel as f64/bins as f64*window)-shift,to_seconds(start+(pixel+1) as f64/bins as f64*window)-shift);let x=r.left()+pixel as f32;painter.line_segment([egui::pos2(x,r.center().y-high*40.),egui::pos2(x,r.center().y-low*40.)],Stroke::new(1.0_f32,Color32::from_rgba_unmultiplied(108,231,198,70)));}}
            let division=self.options.lyric_snap_division.max(1).min(16) as f64;for i in 0..=(window*division).ceil() as i64{let beat=start+i as f64/division;let x=r.left()+((beat-start)/window) as f32*r.width();let whole=(beat-beat.round()).abs()<1e-6;let (measure,in_bar)=timeline.as_ref().map(|t|t.position((beat*ppq).round() as i64)).unwrap_or(((beat/4.).floor() as i64+1,beat%4.+1.));let bar=(in_bar-1.).abs()<1e-6;painter.line_segment([egui::pos2(x,r.top()),egui::pos2(x,r.bottom())],Stroke::new(if bar{1.5_f32}else{1.0_f32},if bar{Color32::from_gray(110)}else if whole{Color32::from_gray(70)}else{Color32::from_gray(37)}));if whole{painter.text(egui::pos2(x+2.,r.bottom()-14.),egui::Align2::LEFT_TOP,format!("{measure}:{in_bar:.0}"),egui::FontId::proportional(9.),MUTED);}}
            if let Ok(doc)=&document{let shift=self.options.lyric_offset as i64+self.options.audio_alignment_ms();for e in &doc.entries{let seconds=(e.ms+shift).max(0) as f64/1000.;let beat=to_beat(seconds);if beat>=start&&beat<start+window{let x=r.left()+((beat-start)/window) as f32*r.width();painter.line_segment([egui::pos2(x,r.top()+5.),egui::pos2(x,r.bottom()-20.)],Stroke::new(1.0_f32,ACCENT));painter.text(egui::pos2(x+3.,r.top()+7.),egui::Align2::LEFT_TOP,&e.text,egui::FontId::proportional(11.),Color32::WHITE);}}let current=doc.entries.iter().rev().find(|e|((e.ms+shift).max(0) as f64/1000.)<=self.chart_cursor);if let Some(e)=current{ui.label(RichText::new(&e.text).size(20.).color(ACCENT));}}
            let x=r.left()+((to_beat(self.chart_cursor)-start)/window) as f32*r.width();painter.line_segment([egui::pos2(x,r.top()),egui::pos2(x,r.bottom())],Stroke::new(2.0_f32,Color32::WHITE));if response.clicked(){if let Some(pos)=response.interact_pointer_pos(){self.chart_cursor=crate::lyrics::snap_seconds(to_seconds(start+(pos.x-r.left()) as f64/r.width() as f64*window),self.score.as_ref(),&self.options).min(end);let playing=self.preview_audio.player.playing();self.preview_audio.player.stop();if playing{self.play_lyrics();}}}
        });ui.add_space(16.);
        if !self.options.lyric_repairs.is_empty(){card(ui,|ui|{kicker(ui,"TIMESTAMPS TO REVIEW");ui.label(RichText::new(format!("{} backward timestamps were automatically spaced between the preceding and following lyrics.",self.options.lyric_repairs.len())).color(Color32::from_rgb(246,164,119)));ui.label(RichText::new("Review these against the audio. You can adjust their times below later; this notice is saved with the project and export.").color(MUTED).size(12.));egui::CollapsingHeader::new("Show original and repaired times").show(ui,|ui|{for r in &self.options.lyric_repairs{ui.label(format!("Line {} · {} · {} -> {}",r.line,r.text,crate::lyrics::stamp(r.original_ms),crate::lyrics::stamp(r.repaired_ms)));}});});ui.add_space(16.);}
        card(ui,|ui|{kicker(ui,"LYRIC TEXT & TIMESTAMPS");match document{Ok(mut doc)=>{
                ui.label(RichText::new(if doc.enhanced{"Enhanced LRC: words retain their order and share their source line's phrase."}else{"Standard LRC: each phrase ends one chart tick before the next phrase starts."}).color(MUTED).size(12.));
                let mut changed=false;egui::ScrollArea::vertical().id_salt("lyric-rows").max_height(240.).show(ui,|ui|{egui::Grid::new("lyric_times").striped(true).show(ui,|ui|{ui.strong("File time (s)");ui.strong("Lyric");ui.strong("Align / Preview");ui.end_row();for e in &mut doc.entries{let mut seconds=e.ms as f64/1000.;if ui.add(egui::DragValue::new(&mut seconds).speed(0.01).range(0.0..=86399.0).fixed_decimals(3)).changed(){let shift=self.options.lyric_offset as i64+self.options.audio_alignment_ms();let aligned=crate::lyrics::snap_seconds(seconds+shift as f64/1000.,self.score.as_ref(),&self.options);e.ms=((aligned*1000.).round() as i64-shift).max(0);changed=true;}if ui.add(egui::TextEdit::singleline(&mut e.text).desired_width(360.)).changed(){changed=true;}ui.horizontal(|ui|{if ui.small_button("To playhead").on_hover_text("Move this lyric to the current playhead, snapping when enabled.").clicked(){let shift=self.options.lyric_offset as i64+self.options.audio_alignment_ms();let at=crate::lyrics::snap_seconds(self.chart_cursor,self.score.as_ref(),&self.options);e.ms=((at*1000.).round() as i64-shift).max(0);changed=true;}if ui.small_button("Snap").on_hover_text("Snap this lyric to the closest selected grid line.").clicked(){let shift=self.options.lyric_offset as i64+self.options.audio_alignment_ms();let mut options=self.options.clone();options.lyric_snap=true;let at=crate::lyrics::snap_seconds((e.ms+shift) as f64/1000.,self.score.as_ref(),&options);e.ms=((at*1000.).round() as i64-shift).max(0);changed=true;}if ui.small_button("Seek").clicked(){self.preview_audio.player.stop();self.chart_cursor=(e.ms+self.options.lyric_offset as i64+self.options.audio_alignment_ms()).max(0) as f64/1000.;}});ui.end_row();}});});
                if changed{self.options.lyric_text=Some(doc.lrc(0));}
                if !doc.repairs.is_empty()&&ui.button("Use repaired timestamps in editor").clicked(){self.options.lyric_text=Some(doc.lrc(0));}
            },Err(e)=>{ui.label(RichText::new(e).color(Color32::from_rgb(246,164,119)));}}
            egui::CollapsingHeader::new("Edit LRC text directly").show(ui,|ui|{if self.options.lyric_text.is_none(){if let Some(path)=&self.options.lyrics{self.options.lyric_text=std::fs::read_to_string(path).ok();}}if let Some(text)=&mut self.options.lyric_text{ui.add(egui::TextEdit::multiline(text).desired_rows(10).desired_width(ui.available_width()).font(egui::TextStyle::Monospace));}else{ui.label("Open an LRC file or choose New lyrics.");}});
        });
    }
    fn video_picker(&mut self,ui:&mut egui::Ui){
        let before=self.options.music_video.clone();file_picker(ui,"Music video",&mut self.options.music_video,engine::allowed("video"));
        if before!=self.options.music_video{if self.options.music_video.is_some(){self.options.extras.remove("Video background");}self.preview_audio.sync(self.options.recording_source());self.chart_cursor=0.;self.wave_drag=None;}
        if self.options.music_video.is_some(){if self.preview_audio.loading(){ui.horizontal(|ui|{ui.spinner();ui.label("Extracting video audio and preparing its waveform…");});}if let Some(error)=&self.preview_audio.error{ui.label(RichText::new(format!("Music video audio: {error}")).color(Color32::from_rgb(246,99,110)));}ui.label(RichText::new(format!("Using the video's audio · Audio alignment {:+} ms · Linked video start {} ms",self.options.audio_alignment_ms(),-self.options.audio_alignment_ms())).color(ACCENT));ui.label(RichText::new("Audio is extracted for preview and exported as song.wav. Audio alignment also sets the video's playback offset. Clear Music video to use the separate recording again.").size(12.).color(MUTED));}
    }
    fn stem_controls(&mut self,ui:&mut egui::Ui){
        card(ui,|ui|{kicker(ui,"LOCAL STEM SEPARATION");ui.label(RichText::new("One song. Four instrument stems.").size(18.).strong());
        file_picker(ui,"Song recording",&mut self.options.audio,&["ogg","opus","mp3","wav"]);
        if let Some(video)=&self.options.music_video{ui.label(RichText::new(format!("Active source: music video {}",video.file_name().unwrap_or_default().to_string_lossy())).color(ACCENT));}
        ui.label("Separate the selected recording or music video's audio into Vocals, Guitar, Bass and Drums, then assign them below. Piano and other accompaniment become the backing mix. Preview keeps your original recording; all exported stems share its alignment.");
        ui.label(RichText::new("Requires Python 3.10 or 3.11. Install stem tools downloads a private CPU Demucs environment; the first separation downloads model weights. Audio stays on this computer. Guitar separation is experimental—listen to each result.").size(12.).color(MUTED));
        file_picker(ui,"Stem tools Python",&mut self.options.stem_python,&["exe"]);
        ui.label(RichText::new("Leave Python unselected to use your installed python. Setup and separation save files in the folder you choose; source files remain intact.").size(12.).color(MUTED));
        let busy=self.job.is_some();ui.horizontal(|ui|{
          if ui.add_enabled(!busy,egui::Button::new("Install stem tools")).on_hover_text("Create a private environment using Python 3.10/3.11 and download CPU Demucs dependencies. Choose a folder with several GB free.").clicked(){
            if let Some(folder)=rfd::FileDialog::new().set_title("Choose stem tools folder").pick_folder(){let python=self.options.stem_python.clone().unwrap_or_else(||PathBuf::from("python"));let log=folder.join("fretforge-stem-setup.log");self.stem_log=Some(log.clone());let(tx,rx)=mpsc::channel();self.job=Some(rx);self.status="Installing local stem tools…".into();self.error=false;thread::spawn(move||{let _=tx.send(Job::StemSetup(crate::stems::setup(&python,&folder,&log)));});}
          }
          if ui.add_enabled(!busy&&self.options.recording_source().is_some(),egui::Button::new("Separate song & assign stems")).on_hover_text("Run six-source Demucs locally. This can take several minutes. Replace the four instrument stems and clear Rhythm, Keys and split drum stems only after every output succeeds; their sounds are included in the generated stems and backing mix.").clicked(){
            if let Some(folder)=rfd::FileDialog::new().set_title("Choose separated stems folder").pick_folder(){let source=self.options.recording_source().as_ref().unwrap().clone();let python=self.options.stem_python.clone().unwrap_or_else(||PathBuf::from("python"));let log=folder.join("fretforge-stem-separation.log");self.stem_log=Some(log.clone());let(tx,rx)=mpsc::channel();self.job=Some(rx);self.status="Separating song locally; preparing model and instrument stems…".into();self.error=false;thread::spawn(move||{let _=tx.send(Job::Separated(crate::stems::separate(&python,&source,&folder,&log)));});}
          }
          if ui.add_enabled(!busy&&self.options.separated.is_some(),egui::Button::new("Clear separated set")).on_hover_text("Unassign this generated set and restore the full recording for export. Saved stem files remain available.").clicked(){crate::stems::clear(&mut self.options);}
        });
        if busy&&self.stem_log.is_some(){ui.horizontal(|ui|{ui.spinner();ui.label("Stem tools are working… this can take several minutes.");});}
        if let Some(log)=&self.stem_log{let tail=crate::stems::log_tail(log);if !tail.is_empty(){ui.label(RichText::new(tail).size(11.).color(MUTED));}ui.label(RichText::new(format!("Log: {}",log.display())).size(11.).color(MUTED));}
        if let Some(s)=&self.options.separated{ui.label(RichText::new(format!("Export backing: {}",s.backing.display())).size(12.).color(ACCENT));if let Err(e)=s.validate(&self.options){ui.label(RichText::new(e).color(Color32::from_rgb(246,99,110)));}}
        });
    }
    fn media(&mut self,ui:&mut egui::Ui){self.stem_controls(ui);ui.add_space(16.);card(ui,|ui|{kicker(ui,"MUSIC VIDEO & LINKED AUDIO");self.video_picker(ui);});ui.add_space(16.);card(ui,|ui|{kicker(ui,"ARTWORK & PRESENTATION");file_picker(ui,"Cover artwork",&mut self.options.cover,&["png","jpg","jpeg"]);ui.horizontal(|ui|{ui.label("Preview start (ms)");ui.add(egui::DragValue::new(&mut self.options.preview_start).range(0..=3600000));ui.checkbox(&mut self.options.video_loop,"Loop background video").on_hover_text("Repeat the selected background/music video when it finishes before the song ends.");});});ui.add_space(16.);
        card(ui,|ui|{kicker(ui,"EXTRAS & STEMS");ui.label(RichText::new("Add atmosphere. Keep every part in sync.").size(18.).strong());ui.label(RichText::new("Aligned recordings and full-length stems export as WAV with matching silence or trimming. Other media is copied intact. Custom highways, icons and colors include installation instructions.").color(MUTED).size(12.));
            for (label,_,kind) in engine::EXTRA_TYPES{let mut path=self.options.extras.get(label).cloned();file_picker(ui,label,&mut path,engine::allowed(kind));if let Some(p)=path{self.options.extras.insert(label.into(),p);}else{self.options.extras.remove(label);}}
        });}
    fn export_page(&mut self,ui:&mut egui::Ui){
        card(ui,|ui|{kicker(ui,"EXPORT DESTINATION");ui.horizontal(|ui|{ui.label(if self.options.output.as_os_str().is_empty(){"Choose where your song folder will be created.".into()}else{self.options.output.display().to_string()});if ui.button("Choose folder").clicked(){if let Some(p)=rfd::FileDialog::new().pick_folder(){self.options.output=p;}}});});ui.add_space(16.);
        card(ui,|ui|{kicker(ui,"PACKAGE SUMMARY");if let Some(s)=&self.score{ui.label(RichText::new(if self.options.title.is_empty(){"Untitled"}else{&self.options.title}).size(24.).strong());ui.label(RichText::new(&self.options.artist).color(MUTED));ui.add_space(12.);
            for (i,name) in ROLES.iter().enumerate(){if self.options.tracks[i].is_some(){ui.horizontal(|ui|{ui.label(RichText::new("+").color(ACCENT));ui.label(*name);ui.label(RichText::new(self.options.tracks[i].and_then(|ix|s.tracks.iter().find(|t|t.index==ix)).map(|t|t.name.as_str()).unwrap_or("Missing track")).color(MUTED));});}}
            
            ui.add_space(10.);ui.label(format!("{} difficulties · {:.1} BPM · Star Power {}",self.options.difficulties.iter().filter(|v|**v).count(),self.options.bpm,if self.options.star_power{"on"}else{"off"}));
            ui.label(format!("All parts: {:+.2} bars · Audio alignment: {:+} ms",self.options.total_chart_quarters() as f64/4.,self.options.audio_alignment_ms()));ui.label(RichText::new("notes.chart  /  lyrics.lrc  /  song.ini  /  audio  /  conversion report").color(MUTED).size(12.));
            match engine::build_chart(s,&self.options){Ok((_,report))=>{ui.add_space(10.);ui.label(RichText::new("Chart validation passed").color(ACCENT));if report["lyrics"].as_u64().unwrap_or(0)>0{ui.label(format!("{} timed lyric events",report["lyrics"]));}let repairs=report["lyric_repairs"].as_array().map(|a|a.len()).unwrap_or(0);if repairs>0{ui.label(RichText::new(format!("{repairs} repaired lyric timestamps: review in Lyrics or LYRIC-REVIEW.txt.")).color(Color32::from_rgb(246,164,119)));}},Err(e)=>{ui.add_space(10.);ui.label(RichText::new(e).color(Color32::from_rgb(246,164,119)));}}
        }else{ui.label("Import a score to prepare your export.");}});ui.add_space(16.);
        card(ui,|ui|{ui.label(RichText::new("One more step before the spotlight.").size(19.).strong());ui.label(RichText::new("These are starter charts. Open the export in Moonscraper, align it to the recording, review every part and difficulty, then playtest in Clone Hero.").color(MUTED));ui.add_space(14.);
            ui.add_enabled_ui(self.score.is_some()&&self.job.is_none()&&!self.options.output.as_os_str().is_empty(),|ui|{if primary(ui,"Create song folder"){self.start_export();}});
            if let Some(path)=&self.exported{ui.add_space(10.);ui.label(RichText::new("Your song folder is ready.").color(ACCENT));if ui.button("Open exported folder").clicked(){let program=if cfg!(windows){"explorer.exe"}else if cfg!(target_os="macos"){"open"}else{"xdg-open"};if let Err(error)=std::process::Command::new(program).arg(path).spawn(){self.status=format!("Could not open the folder: {error}");self.error=true;}}}
        });
    }
    fn guide(&mut self,ui:&mut egui::Ui){ui.add(egui::TextEdit::singleline(&mut self.help_search).hint_text("Search the guide…").desired_width(ui.available_width()));ui.add_space(16.);
        for (title,body) in [
            ("LRC lyrics and alignment","Use Lyrics to open or create standard/enhanced LRC files, edit words/times and save aligned copies. Standard phrases end one tick before the next phrase starts; the last phrase reaches the chart end. Enhanced words share their source line's phrase. Backward timestamp runs are placed evenly between the preceding valid lyric and next later lyric, preserving word order. Review notices are retained in saved projects, conversion-report.json and LYRIC-REVIEW.txt. Without a following anchor, add a later lyric to allow repair. Seek to the first sung word and click Align first lyric to playhead, type First lyric at, or nudge all lyrics by 100 ms. These controls move lyrics while charted notes and audio stay fixed. The original LRC is preserved; exports use the edited and repaired timings. Pitched YARG vocals and notes.mid export have been removed."),
            ("Start with a local score","Choose a GP3, GP4, GP5, GPX or GP7/8 file. The bundled alphaTab reader expands repeats, tuplets and playback techniques. Files are processed locally. Drag and drop a score anywhere in the window."),
            ("Select your lineup","Choose a percussion track for Drums and pitched tracks for Guitar, Bass and Rhythm. Uncheck any part you do not need. Tracks can be reused across pitched roles. Choose a difficulty, step the preview by a quarter beat, and zoom independently in each preview. Shift all charted parts together by whole or quarter bars."),
            ("Drum mapping","Map each named MIDI sound to a drum lane or Ignore. The live collision summary beneath the pattern preview updates after every reassignment, showing all Expert collision locations and whether the overlap count improved or increased. Expand details to identify the source sounds and jump to a collision. Orange rings mark conflicts in the drum preview. Strict export rejects collisions, while the visual preview remains available. Cymbal markers are preserved on Hard and Expert. Written accents and ghosts are preserved on Expert; optional velocity inference adds estimates. At BPM above 110, consecutive Expert kicks within a sixteenth note alternate normal and 2× kick."),
            ("Star Power placement","Phrases normally last two measures and adapt from one to four as note density changes. The planner leaves four full measures after a phrase before another can begin, targets roughly eight measures / 15 seconds, and favors section boundaries, rhythm changes and strong downbeats or accents. Gold bands mark phrases in the preview; timing conflicts and short endings appear below it and in CHARTING-REVIEW.txt. The 30-second target cannot always coexist with four-measure spacing at very slow tempos or long rests. Two collected phrases reach activation; the game controls meter gain and drain, which this app does not change."),
            ("GHL note shapes and patterns","The All Notes and Common Patterns reference distinguishes 64 mathematical button states from playable chart choices. Prefer neighboring two-button chords and readable diagonals; the mapper penalizes outer jumps and crowded three-button grips. It recognizes repeated picking, gallops, trills, row flips, zigzags, staircases, triplets and chord shifts in the review. Optional simple barres in Song details apply to uncrowded Hard/Expert dyads, display as squares, and receive play-test notices. Easy/Medium simplify these grips. Open is always separate from fretted chords; advanced barre-plus-fret shapes and four-to-six-button clusters are not generated. The complete reference is included as GHL_All_Notes_and_Common_Patterns.md."),
            ("Six-fret GHL parts","Enable GHL Guitar, GHL Bass or GHL Rhythm in the lineup. Literal 6-fret v1.0 maps entire phrases using six-fret templates and progression-aware chords, with up to three buttons and no same-column barres by default. Repeated and transposed riffs reuse shapes. Open pedal tones require repeated low picked notes; open strings alone stay fretted. Long phrases reuse buttons at position resets without losing attacks. Expert preserves significant three-voice chords even at high speed. Unspecified articulation defaults to strum and is reported for review. Every part shares the chart tempo and note offset. Easy and Medium are reduced from the Expert arrangement. Review all parts against the audio."),
            ("Five-fret reductions","Guitar, Rhythm and Bass follow Literal 5-fret v1.0. Expert retains every distinct source attack, uses phrase contour and recurring chord shapes, and preserves explicit legato/tapping technique. Easy uses G/R/Y singles; Medium uses G/R/Y/B and at most two-note chords; Hard uses all colors without taps or opens. Lower tiers retain rhythmic anchors and contour landmarks at their original times. Open strings remain colored notes; only explicit Expert gameplay annotations can become open bass. Expert preserves individual source releases, trimming only same-button/open conflicts; lower difficulties use beat and millisecond clearance. Transposed motifs reuse mappings. Narrow melodies use compact spans. All reductions, inferences and resets are listed in conversion-report.json. Review warnings under the preview and in CHARTING-REVIEW.txt; spacing targets are recommendations, not engine limits. The score cannot prove what is audible: compare every part to the recording."),
            ("Timing and lyrics","Change the overall chart BPM on Score & tracks or in Song details. The controls share one setting for every instrument and difficulty, exported tempo, lyric timing and ratings. Audio recordings are not stretched. Check Constant BPM beside the BPM box to override all Guitar Pro tempo changes and export exactly one tempo flag at tick zero. Unchecked, Overall BPM scales the source tempo map relative to its starting tempo. Playback, lyrics, ratings and review findings share the selected mode. Time signatures are preserved. Attach a recording in Pattern preview and press Play. The playhead follows the recording across sixteen-beat windows. Drag the waveform or type its alignment in milliseconds: right adds silence, left trims the start of the exported audio. The same alignment is baked into all full-length stems; song.ini delay stays zero. The arrow buttons move every instrument and difficulty together by one starting-meter bar (the same tick offset for every part). Sections and Star Power move with the notes; recording-based LRC lyrics follow audio alignment instead. Standard and enhanced LRC timestamps are converted to chart ticks; lyrics offsets and embedded LRC offsets are applied. Negative lyric timestamps clamp to zero. Review phrase boundaries and timing in Moonscraper."),
            ("Ratings and Star Power","Auto intensity is a Rust note-density heuristic on a 0–6 scale, not an official difficulty rating. Override each part or the overall rating. Star Power uses two-bar phrases in populated eight-bar windows. Easy, Medium and Hard reductions use progressively tighter rhythmic spacing."),
            ("Media and extras","Aligned main audio and full-length stems are exported as 24-bit WAV, keeping their original sample rate and channels. Unaligned recordings, preview excerpts, cover art and photo/video backgrounds are copied intact. Use stems synchronized to the backing recording. Do not combine a complete drum stem with numbered drum stems. Custom highways, color profiles and game icons are packaged in Extras/Custom and need separate installation."),
            ("Export and review","Choose an existing output folder. Export writes a new uniquely named song folder and never replaces an existing song. You get notes.chart, song.ini, selected media and a conversion report. Without a recording, a silent practice WAV is generated. Align and review in Moonscraper, scan songs in Clone Hero and playtest."),
            ("Save your workspace","Save project stores the source path and your settings in a JSON project file. Open project reloads that source and restores your choices. Keep the score and attached media at their original paths, or update the project if you move them."),
        ]{if self.help_search.is_empty()||format!("{title} {body}").to_lowercase().contains(&self.help_search.to_lowercase()){card(ui,|ui|{ui.label(RichText::new(title).size(18.).strong());ui.label(RichText::new(body).color(MUTED));});ui.add_space(12.);}}
    }
}
type BTreeMapCompat=std::collections::BTreeMap<i32,usize>;
impl eframe::App for Studio {
    fn update(&mut self,ctx:&egui::Context,_:&mut eframe::Frame){self.poll();self.preview_audio.sync(self.options.recording_source());let previous_bpm=(self.options.bpm,self.options.constant_bpm);
        if let Some(position)=self.preview_audio.player.position(){self.chart_cursor=position;}
        if self.preview_audio.player.playing(){ctx.request_repaint_after(std::time::Duration::from_millis(16));}
        if self.job.is_some()||self.preview_audio.loading(){ctx.request_repaint_after(std::time::Duration::from_millis(80));}
        if std::env::args().any(|a|a=="--capture"){
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            if self.score.is_some() && self.job.is_none()&&!self.preview_audio.loading(){self.capture_frames+=1;if self.capture_frames==15{ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));}}
            let screenshot=ctx.input(|i|i.events.iter().find_map(|e|if let egui::Event::Screenshot{image,..}=e{Some(image.clone())}else{None}));
            if let Some(img)=screenshot{let bytes:Vec<u8>=img.pixels.iter().flat_map(|p|p.to_array()).collect();let _=image::save_buffer(engine::asset_root().join(if std::env::args().any(|a|a=="--review-preview"){"charting-preview.png".to_string()}else if std::env::args().any(|a|a=="--demo-audio"){"pattern-preview.png".to_string()}else if std::env::args().any(|a|a=="--demo-drums"){"drum-mapping-preview.png".to_string()}else if std::env::args().any(|a|a=="--demo-ghl"){"ghl-preview.png".to_string()}else if self.page==0{"studio-preview.png".to_string()}else{format!("studio-preview-{}.png",self.page)}),&bytes,img.size[0] as u32,img.size[1] as u32,image::ColorType::Rgba8);ctx.send_viewport_cmd(egui::ViewportCommand::Close);}
        }
        if self.job.is_none(){let path=ctx.input(|i|i.raw.dropped_files.first().and_then(|f|f.path.clone()));if let Some(path)=path{self.load(path)}}
        self.sidebar(ctx);
        egui::TopBottomPanel::bottom("status").frame(egui::Frame::new().fill(Color32::from_rgb(17,22,32)).inner_margin(12.)).show(ctx,|ui|{ui.horizontal(|ui|{ui.label(RichText::new(if self.error{"!"}else{"+"}).color(if self.error{Color32::from_rgb(247,139,126)}else{ACCENT}));ui.label(RichText::new(&self.status).size(12.).color(if self.error{Color32::from_rgb(247,139,126)}else{MUTED}));});});
        egui::CentralPanel::default().frame(egui::Frame::new().fill(BG).inner_margin(30.)).show(ctx,|ui|{self.header(ui);let mut scroll=egui::ScrollArea::vertical().id_salt("page-scroll");if self.page==0&&std::env::args().any(|a|a=="--capture")&&(std::env::args().any(|a|a=="--demo-drums"||a=="--demo-ghl")||std::env::args().any(|a|a=="--review-preview")){scroll=scroll.vertical_scroll_offset(if std::env::args().any(|a|a=="--demo-ghl"){600.}else if std::env::args().any(|a|a=="--demo-drums"){900.}else{420.});}scroll.show(ui,|ui|{ui.set_min_width(ui.available_width());match self.page{0=>self.source_page(ui),1=>self.details(ui),2=>self.media(ui),3=>self.export_page(ui),5=>self.lyrics_page(ui),_=>self.guide(ui)}ui.add_space(20.);});});
        if previous_bpm!=(self.options.bpm,self.options.constant_bpm){if let Some(score)=self.score.clone(){self.retime_playback(&score);}ctx.request_repaint();}
    }
}
fn card<R>(ui:&mut egui::Ui,add:impl FnOnce(&mut egui::Ui)->R)->R{egui::Frame::new().fill(CARD).stroke(Stroke::new(1.0_f32,LINE)).corner_radius(12.).inner_margin(18.).show(ui,|ui|{ui.set_min_width(ui.available_width());add(ui)}).inner}
fn kicker(ui:&mut egui::Ui,text:&str){ui.label(RichText::new(text).size(10.).color(ACCENT).strong());ui.add_space(4.);}
fn primary(ui:&mut egui::Ui,text:&str)->bool{ui.add(egui::Button::new(RichText::new(text).color(Color32::from_rgb(15,37,33)).strong()).fill(ACCENT).corner_radius(7.)).clicked()}
fn stat(ui:&mut egui::Ui,label:&str,value:&str){ui.vertical(|ui|{ui.set_min_width(145.);ui.label(RichText::new(label).size(10.).color(MUTED));ui.label(RichText::new(value).size(22.).strong());});}
fn bpm_editor(ui:&mut egui::Ui,bpm:&mut f64,constant:&mut bool){ui.horizontal(|ui|{if ui.add(egui::DragValue::new(bpm).range(0.001..=1000.).speed(0.1).max_decimals(3).suffix(" BPM")).on_hover_text("Click to type or drag to adjust. Applies to every instrument, lyric timing, ratings and export. Audio is not stretched.").changed(){ui.ctx().request_repaint();}if ui.checkbox(constant,"Constant BPM").on_hover_text("Override every Guitar Pro tempo change with one BPM flag at the beginning of the chart, using the entered BPM. Playback and export use the same constant tempo.").changed(){ui.ctx().request_repaint();}});}
fn field(ui:&mut egui::Ui,label:&str,value:&mut String){ui.horizontal(|ui|{ui.add_sized([135.,30.],egui::Label::new(RichText::new(label).color(MUTED)));ui.add(egui::TextEdit::singleline(value).desired_width((ui.available_width()-5.).max(200.)));});}
fn rating_combo(ui:&mut egui::Ui,id:impl std::hash::Hash,value:&mut i32){egui::ComboBox::from_id_salt(id).selected_text(if *value<0{"Auto".into()}else{format!("{} / 6",value)}).width(95.).show_ui(ui,|ui|{ui.selectable_value(value,-1,"Auto");for i in 0..=6{ui.selectable_value(value,i,format!("{i} / 6"));}});}
fn file_picker(ui:&mut egui::Ui,label:&str,path:&mut Option<PathBuf>,extensions:&[&str]){ui.horizontal(|ui|{ui.add_sized([160.,30.],egui::Label::new(RichText::new(label).color(MUTED)));let filename=path.as_ref().and_then(|p|p.file_name()).map(|p|p.to_string_lossy().into_owned()).unwrap_or("Choose file…".into());if ui.button(filename).on_hover_text(path.as_ref().map(|p|p.display().to_string()).unwrap_or_default()).clicked(){if let Some(p)=rfd::FileDialog::new().add_filter(label,extensions).pick_file(){*path=Some(p)}}if path.is_some()&&ui.small_button("×").clicked(){*path=None;}});}
fn lane_name(lane:i32)->&'static str{match lane{-1=>"Ignore",0=>"Kick",1=>"Snare · red",2=>"High tom · yellow",3=>"Mid tom · blue",4=>"Low tom · green",66=>"Hi-hat · yellow cymbal",67=>"Ride · blue cymbal",68=>"Crash · green cymbal",_=>"Ignore"}}

/// General MIDI percussion names describe the source hit, independently of its chart lane.
fn midi_drum_name(pitch: i32) -> &'static str {
    match pitch {
        35 => "Acoustic bass drum",
        36 => "Bass drum / kick",
        37 => "Side stick",
        38 => "Acoustic snare",
        39 => "Hand clap",
        40 => "Electric snare",
        41 => "Low floor tom",
        42 => "Closed hi-hat",
        43 => "High floor tom",
        44 => "Pedal hi-hat",
        45 => "Low tom",
        46 => "Open hi-hat",
        47 => "Low-mid tom",
        48 => "High-mid tom",
        49 => "Crash cymbal 1",
        50 => "High tom",
        51 => "Ride cymbal 1",
        52 => "Chinese cymbal",
        53 => "Ride bell",
        54 => "Tambourine",
        55 => "Splash cymbal",
        56 => "Cowbell",
        57 => "Crash cymbal 2",
        58 => "Vibraslap",
        59 => "Ride cymbal 2",
        60 => "High bongo",
        61 => "Low bongo",
        62 => "Mute high conga",
        63 => "Open high conga",
        64 => "Low conga",
        65 => "High timbale",
        66 => "Low timbale",
        67 => "High agogo",
        68 => "Low agogo",
        69 => "Cabasa",
        70 => "Maracas",
        71 => "Short whistle",
        72 => "Long whistle",
        73 => "Short guiro",
        74 => "Long guiro",
        75 => "Claves",
        76 => "High wood block",
        77 => "Low wood block",
        78 => "Mute cuica",
        79 => "Open cuica",
        80 => "Mute triangle",
        81 => "Open triangle",
        _ => "Unrecognized percussion",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn bpm_checkbox_toggles_constant_tempo_beside_the_number(){
        let ctx=egui::Context::default();let mut options=Options::default();
        let frame=|options:&mut Options,events:Vec<egui::Event>|ctx.run(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(1240.,980.))),events,..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|bpm_editor(ui,&mut options.bpm,&mut options.constant_bpm));});
        let output=frame(&mut options,vec![]);let checkbox=output.shapes.iter().find_map(|s|if let egui::epaint::Shape::Text(t)=&s.shape{if t.galley.text()=="Constant BPM"{Some(t.pos+t.galley.size()/2.)}else{None}}else{None}).unwrap();
        let pointer=|pressed|egui::Event::PointerButton{pos:checkbox,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE};frame(&mut options,vec![egui::Event::PointerMoved(checkbox),pointer(true)]);frame(&mut options,vec![pointer(false)]);assert!(options.constant_bpm);assert_eq!(options.bpm,120.);frame(&mut options,vec![pointer(true)]);frame(&mut options,vec![pointer(false)]);assert!(!options.constant_bpm);
    }
    #[test]fn ghl_preview_labels_six_buttons_and_invalidates_settings(){
        let score=Score{ppq:480,end:7680.,tracks:vec![engine::Track{index:0,name:"Guitar".into(),percussion:false,notes:(0..6).map(|i|engine::Note{tick:(i*120) as f64,pitch:60+i,..Default::default()}).collect()}],..Default::default()};
        let mut studio=Studio{score:Some(score.clone()),source:None,options:Options{tracks:[None,None,None,None,Some(0),Some(0),Some(0)],..Default::default()},page:0,role:4,tier:3,preview_start:0.,status:String::new(),error:false,job:None,exported:None,help_search:String::new(),capture_frames:0,collision_snapshot:None,preview_audio:audio::PreviewAudio::default(),chart_cursor:0.,volume:0.6,wave_drag:None,pattern_cache:None,stem_log:None};
        let ctx=egui::Context::default();let output=ctx.run(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(1240.,980.))),..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|studio.preview(ui,&score));});
        let text=output.shapes.into_iter().filter_map(|shape|if let egui::epaint::Shape::Text(t)=shape.shape{Some(t.galley.text().to_owned())}else{None}).collect::<Vec<_>>().join("\n");for label in ["GHL Guitar","Literal 6-fret v1.0","W1","W2","W3","B1","B2","B3","OPEN","6 playable events"]{assert!(text.contains(label),"Missing {label}");}
        studio.options.ghl_open_pedals=false;studio.options.ghl_octave_folding=true;studio.preview_notes(&score).unwrap();assert!(studio.pattern_cache.as_ref().unwrap().key.ghl_fold);assert!(!studio.pattern_cache.as_ref().unwrap().key.ghl_open);
    }
    #[test]fn ghl_barres_render_as_squares_and_setting_changes_invalidate_preview(){
        let score=Score{ppq:480,end:7680.,tracks:vec![engine::Track{index:0,name:"Guitar".into(),percussion:false,notes:(0..3).flat_map(|i|[60,72].map(move|pitch|engine::Note{tick:(i*480) as f64,pitch,length:360.,picked:true,..Default::default()})).collect()}],..Default::default()};
        let mut studio=Studio{score:Some(score.clone()),source:None,options:Options{ghl_simple_barres:true,tracks:[None,None,None,None,Some(0),Some(0),Some(0)],..Default::default()},page:0,role:4,tier:3,preview_start:0.,status:String::new(),error:false,job:None,exported:None,help_search:String::new(),capture_frames:0,collision_snapshot:None,preview_audio:audio::PreviewAudio::default(),chart_cursor:0.,volume:0.6,wave_drag:None,pattern_cache:None,stem_log:None};
        let ctx=egui::Context::default();let output=ctx.run(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(1240.,980.))),..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|studio.preview(ui,&score));});
        assert_eq!(studio.pattern_cache.as_ref().unwrap().barres.len(),3);let squares=output.shapes.iter().filter(|shape|matches!(&shape.shape,egui::epaint::Shape::Rect(r) if (r.rect.width()-12.).abs()<0.01&&(r.rect.height()-12.).abs()<0.01)).count();assert_eq!(squares,6);studio.options.ghl_simple_barres=false;studio.preview_notes(&score).unwrap();assert!(studio.pattern_cache.as_ref().unwrap().barres.is_empty());assert!(!studio.pattern_cache.as_ref().unwrap().key.ghl_barres);
    }
    #[test]fn pitched_preview_shows_rulebook_review_without_deleting_expert_attacks(){
        let score=Score{ppq:480,end:7680.,tracks:vec![engine::Track{index:0,name:"Guitar".into(),percussion:false,notes:(0..4).map(|i|engine::Note{tick:(i*20) as f64,pitch:60+i,..Default::default()}).collect()}],..Default::default()};
        let mut studio=Studio{score:Some(score.clone()),source:None,options:Options{tracks:[None,Some(0),None,None,None,None,None],..Default::default()},page:0,role:1,tier:3,preview_start:0.,status:String::new(),error:false,job:None,exported:None,help_search:String::new(),capture_frames:0,collision_snapshot:None,preview_audio:audio::PreviewAudio::default(),chart_cursor:0.,volume:0.6,wave_drag:None,pattern_cache:None,stem_log:None};
        let ctx=egui::Context::default();let output=ctx.run(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(1240.,980.))),..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|studio.preview(ui,&score));});
        let text=output.shapes.into_iter().filter_map(|shape|if let egui::epaint::Shape::Text(t)=shape.shape{Some(t.galley.text().to_owned())}else{None}).collect::<Vec<_>>().join("\n");assert!(text.contains("Literal 5-fret v1.0"));assert!(text.contains("3 dense Expert notices"));assert!(text.contains("4 playable events"));assert_eq!(studio.pattern_cache.as_ref().unwrap().findings.len(),3);assert!(text.contains("Star Power:"));assert!(!studio.pattern_cache.as_ref().unwrap().star_power.phrases.is_empty());let before=studio.preview_notes(&score).unwrap();studio.options.star_power=false;let after=studio.preview_notes(&score).unwrap();assert_eq!(before.iter().map(|g|(g.tick,g.lane,g.length)).collect::<Vec<_>>(),after.iter().map(|g|(g.tick,g.lane,g.length)).collect::<Vec<_>>());assert!(studio.pattern_cache.as_ref().unwrap().star_power.phrases.is_empty());
    }

    #[test]
    fn preview_stays_visible_and_reports_improvements_and_regressions() {
        let score=Score{ppq:480,end:7680.,tracks:vec![engine::Track{index:0,name:"Drums".into(),percussion:true,notes:vec![38,40].into_iter().map(|pitch|engine::Note{tick:0.,pitch,..Default::default()}).collect()}],..Default::default()};
        let mut studio=Studio{score:Some(score.clone()),source:None,options:Options{tracks:[Some(0),None,None,None,None,None,None],..Default::default()},page:0,role:0,tier:3,preview_start:0.,status:String::new(),error:false,job:None,exported:None,help_search:String::new(),capture_frames:0,collision_snapshot:None,preview_audio:audio::PreviewAudio::default(),chart_cursor:0.,volume:0.6,wave_drag:None,pattern_cache:None,stem_log:None};
        let ctx=egui::Context::default();
        let render=|studio:&mut Studio|{
            let output=ctx.run(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(1240.,980.))),..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|studio.preview(ui,&score));});
            output.shapes.into_iter().filter_map(|shape|if let egui::epaint::Shape::Text(text)=shape.shape{Some(text.galley.text().to_owned())}else{None}).collect::<Vec<_>>().join("\n")
        };
        let initial=render(&mut studio);
        assert!(initial.contains("1 collision location"));
        assert!(initial.contains("1 playable events"));
        assert!(initial.contains("strict export remains blocked"));
        studio.options.drum_map.insert(40,3);
        let improved=render(&mut studio);
        assert!(improved.contains("No drum collisions"));
        assert!(improved.contains("Improved: 1 fewer"));
        studio.options.drum_map.insert(40,1);
        let regressed=render(&mut studio);
        assert!(regressed.contains("Increased: 1 more"));
        studio.options.tracks[0]=None;
        render(&mut studio);
        assert!(studio.collision_snapshot.is_none());
        studio.options.tracks[0]=Some(0);
        render(&mut studio);
        assert_eq!(studio.collision_snapshot.as_ref().unwrap().change,None);
        studio.preview_audio.clip=Some(std::sync::Arc::new(audio::AudioClip::from_samples(vec![0;48000*2*10])));
        let frame=|studio:&mut Studio,events:Vec<egui::Event>|ctx.run(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(1240.,980.))),events,..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|studio.preview(ui,&score));});
        let output=frame(&mut studio,vec![]);
        let rect=output.shapes.iter().find_map(|s|if let egui::epaint::Shape::Rect(r)=&s.shape{if r.fill==Color32::from_rgb(15,20,29){Some(r.rect)}else{None}}else{None}).unwrap();
        let pointer=|pos,pressed|egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE};
        let origin=rect.center();frame(&mut studio,vec![egui::Event::PointerMoved(origin),pointer(origin,true)]);
        frame(&mut studio,vec![egui::Event::PointerMoved(origin+Vec2::new(20.,0.))]);
        frame(&mut studio,vec![egui::Event::PointerMoved(origin+Vec2::new(40.,0.))]);
        let expected=(40./(rect.width()-44.) as f64*8000.).round() as i64;assert_eq!(studio.options.audio_alignment_ms(),expected);
        frame(&mut studio,vec![]);assert_eq!(studio.options.audio_alignment_ms(),expected);
        let output=frame(&mut studio,vec![pointer(origin+Vec2::new(40.,0.),false)]);assert!(studio.wave_drag.is_none());
        let right=output.shapes.iter().find_map(|s|if let egui::epaint::Shape::Text(t)=&s.shape{if t.galley.text()=="1 bar >"{Some(t.pos+t.galley.size()/2.)}else{None}}else{None}).unwrap();
        frame(&mut studio,vec![egui::Event::PointerMoved(right),pointer(right,true)]);frame(&mut studio,vec![pointer(right,false)]);assert_eq!(studio.options.chart_shift_bars,1);
        assert_eq!(studio.preview_notes(&score).unwrap()[0].tick,1920);
    }
    #[test]fn lyric_grid_and_row_playhead_alignment_use_snapped_musical_time(){
        let score=Score{ppq:480,end:7680.,tracks:vec![],..Default::default()};
        let mut studio=Studio{score:Some(score.clone()),source:None,options:Options{lyric_text:Some("[00:00.100]one".into()),lyric_snap:true,lyric_snap_division:4,lyric_offset:100,audio_shift_ms:250,..Default::default()},page:5,role:0,tier:3,preview_start:0.,status:String::new(),error:false,job:None,exported:None,help_search:String::new(),capture_frames:0,collision_snapshot:None,preview_audio:audio::PreviewAudio::default(),chart_cursor:1.18,volume:0.6,wave_drag:None,pattern_cache:None,stem_log:None};
        let ctx=egui::Context::default();let frame=|studio:&mut Studio,events|ctx.run(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(1240.,3000.))),events,..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|studio.lyrics_page(ui));});
        let output=frame(&mut studio,vec![]);let labels=output.shapes.iter().filter_map(|s|if let egui::epaint::Shape::Text(t)=&s.shape{Some(t.galley.text())}else{None}).collect::<Vec<_>>();assert!(labels.contains(&"1/4 beat (sixteenth notes)"));assert!(labels.contains(&"1:1"));
        let pos=output.shapes.iter().find_map(|s|if let egui::epaint::Shape::Text(t)=&s.shape{if t.galley.text()=="To playhead"{Some(t.pos+t.galley.size()/2.)}else{None}}else{None}).unwrap();let event=|pressed|egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE};frame(&mut studio,vec![egui::Event::PointerMoved(pos),event(true)]);frame(&mut studio,vec![event(false)]);
        assert_eq!(crate::lyrics::load(&studio.options).unwrap().entries[0].ms,775);assert_eq!(studio.options.audio_alignment_ms(),250);assert_eq!(studio.options.lyric_offset,100);
        let mut value=0.1234567;let mut width=0.;let _=ctx.run(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(1240.,1000.))),..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|{width=wide_scrubber(ui,&mut value,0.0..=10.,"Playhead (s)",2,None).rect.width();});});assert!(width>800.);assert_eq!(value,0.1234567);
    }
    #[test]fn drum_mapping_rows_change_color_when_ignore_and_collisions_change(){
        let score=Score{ppq:480,end:7680.,tracks:vec![engine::Track{index:0,name:"Drums".into(),percussion:true,notes:vec![engine::Note{tick:90.,pitch:35,..Default::default()},engine::Note{tick:0.,pitch:36,..Default::default()},engine::Note{tick:0.,pitch:38,..Default::default()}]}],..Default::default()};
        let mut options=Options{tracks:[Some(0),None,None,None,None,None,None],strict:false,..Default::default()};options.drum_map.insert(35,-1);options.drum_map.insert(38,0);
        let mut studio=Studio{score:Some(score.clone()),source:None,options,page:0,role:0,tier:3,preview_start:0.,status:String::new(),error:false,job:None,exported:None,help_search:String::new(),capture_frames:0,collision_snapshot:None,preview_audio:audio::PreviewAudio::default(),chart_cursor:0.,volume:0.6,wave_drag:None,pattern_cache:None,stem_log:None};
        let ctx=egui::Context::default();let frame=|studio:&mut Studio,events|ctx.run(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(1240.,3000.))),events,..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|studio.source_page(ui));});
        let output=frame(&mut studio,vec![]);let pos=output.shapes.iter().find_map(|s|if let egui::epaint::Shape::Text(t)=&s.shape{if t.galley.text()=="Drum mapping & dynamics"{Some(t.pos+t.galley.size()/2.)}else{None}}else{None}).unwrap();let event=|pressed|egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE};frame(&mut studio,vec![egui::Event::PointerMoved(pos),event(true)]);frame(&mut studio,vec![event(false)]);let output=frame(&mut studio,vec![]);
        let color=|out:&egui::FullOutput,pitch:i32|out.shapes.iter().find_map(|s|if let egui::epaint::Shape::Text(t)=&s.shape{if t.galley.text().contains(&format!("MIDI {pitch} ·")){Some(t.galley.job.sections[0].format.color)}else{None}}else{None}).unwrap();assert_eq!(color(&output,35),Color32::from_rgb(246,99,110));for p in [36,38]{assert_eq!(color(&output,p),Color32::from_rgb(245,207,92));}
        studio.options.drum_map.insert(38,1);studio.options.drum_map.insert(35,2);let output=frame(&mut studio,vec![]);for p in [35,36,38]{assert_eq!(color(&output,p),Color32::from_rgb(230,237,247));}
    }
    #[test]fn idle_preview_redraws_preserve_audio_clock_without_seeks(){
        let score=Score{ppq:480,end:7680.,tracks:vec![engine::Track{index:0,name:"Guitar".into(),percussion:false,notes:vec![engine::Note{tick:0.,pitch:60,..Default::default()}]}],..Default::default()};
        let mut studio=Studio{score:Some(score.clone()),source:None,options:Options{tracks:[None,Some(0),None,None,None,None,None],pattern_preview_beats:4.,..Default::default()},page:0,role:1,tier:3,preview_start:0.,status:String::new(),error:false,job:None,exported:None,help_search:String::new(),capture_frames:0,collision_snapshot:None,preview_audio:audio::PreviewAudio::default(),chart_cursor:0.,volume:0.6,wave_drag:None,pattern_cache:None,stem_log:None};
        let ctx=egui::Context::default();
        for i in 0..30 {let seconds=0.1234567+i as f64*0.016321;studio.chart_cursor=seconds;studio.preview_start=1.137;
            let _=ctx.run(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(1240.,1600.))),..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|studio.preview(ui,&score));});
            assert_eq!(studio.chart_cursor,seconds,"Idle draw requested a seek by rounding the live audio clock");assert_eq!(studio.preview_start,1.137,"Idle draw rounded the follow window");
            let _=ctx.run(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(1240.,1600.))),..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|studio.lyrics_page(ui));});assert_eq!(studio.chart_cursor,seconds,"Lyrics draw rounded the audio clock");
        }
    }
    #[test]fn lyrics_tab_notifies_repairs_and_aligns_the_start_without_moving_audio_or_notes(){
        let score=Score{ppq:480,end:7680.,tracks:vec![engine::Track{index:0,name:"Drums".into(),percussion:true,notes:vec![engine::Note{tick:0.,pitch:36,..Default::default()}]}],..Default::default()};
        let mut studio=Studio{score:Some(score.clone()),source:None,options:Options{tracks:[Some(0),None,None,None,None,None,None],audio_shift_ms:250,lyric_text:Some("[00:01]<00:01>A <00:00>B <00:03>C".into()),..Default::default()},page:5,role:0,tier:3,preview_start:0.,status:String::new(),error:false,job:None,exported:None,help_search:String::new(),capture_frames:0,collision_snapshot:None,preview_audio:audio::PreviewAudio::default(),chart_cursor:4.,volume:0.6,wave_drag:None,pattern_cache:None,stem_log:None};
        let ctx=egui::Context::default();let frame=|studio:&mut Studio,events:Vec<egui::Event>|ctx.run(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(1240.,1800.))),events,..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|studio.lyrics_page(ui));});
        let output=frame(&mut studio,vec![]);assert_eq!(studio.options.lyric_repairs.len(),1);let text=output.shapes.iter().filter_map(|s|if let egui::epaint::Shape::Text(t)=&s.shape{Some(t.galley.text())}else{None}).collect::<Vec<_>>().join("\n");assert!(text.contains("automatically spaced"));
        let locate=|output:&egui::FullOutput,label:&str|output.shapes.iter().find_map(|s|if let egui::epaint::Shape::Text(t)=&s.shape{if t.galley.text()==label{Some(t.pos+t.galley.size()/2.)}else{None}}else{None}).unwrap();let pos=locate(&output,"Align first lyric to playhead");let pointer=|pos,pressed|egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE};frame(&mut studio,vec![egui::Event::PointerMoved(pos),pointer(pos,true)]);let output=frame(&mut studio,vec![pointer(pos,false)]);assert_eq!(studio.options.lyric_offset,2750);assert_eq!(studio.options.audio_alignment_ms(),250);assert_eq!(engine::gems(&score,&studio.options,0,3).unwrap()[0].tick,0);
        let pos=locate(&output,"+100 ms");frame(&mut studio,vec![egui::Event::PointerMoved(pos),pointer(pos,true)]);frame(&mut studio,vec![pointer(pos,false)]);assert_eq!(studio.options.lyric_offset,2850);assert_eq!(studio.options.audio_alignment_ms(),250);
    }

}

fn preview_zoom(ui:&mut egui::Ui,beats:&mut f32){
    if ui.add_enabled(*beats>1.,egui::Button::new("Zoom in")).clicked(){*beats=(*beats/2.).max(1.);}
    if ui.add_enabled(*beats<64.,egui::Button::new("Zoom out")).clicked(){*beats=(*beats*2.).min(64.);}
    if ui.small_button("Reset zoom").clicked(){*beats=16.;}
    ui.label(format!("{} beats",beats));
}

fn grid_name(division:u32)->&'static str{match division{1=>"Beat (quarter notes)",2=>"1/2 beat (eighth notes)",4=>"1/4 beat (sixteenth notes)",8=>"1/8 beat",16=>"1/16 beat",_=>"1/4 beat (sixteenth notes)"}}
fn wide_scrubber<N:egui::emath::Numeric>(ui:&mut egui::Ui,value:&mut N,range:std::ops::RangeInclusive<N>,label:&str,decimals:usize,step:Option<f64>)->egui::Response{
    ui.scope(|ui|{ui.spacing_mut().slider_width=(ui.available_width()-210.).max(200.);let mut slider=egui::Slider::new(value,range).clamping(egui::SliderClamping::Edits).text(label).fixed_decimals(decimals);if let Some(step)=step{slider=slider.step_by(step);}ui.add(slider)}).inner
}
