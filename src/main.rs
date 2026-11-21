use std::env::{args,var};
use std::fs::{create_dir_all,rename,copy,remove_file,read_dir,write,symlink_metadata};
use std::io::{self,Write};
use std::path::{Path,PathBuf};
use std::sync::atomic::{AtomicUsize,Ordering::Relaxed};
use std::sync::mpsc;
use std::thread;

#[derive(Clone)]struct Q{p:PathBuf,z:u64}
#[derive(Clone,Copy,PartialEq)]enum L{Ru,En}
impl L{fn x<'a>(&self,a:&'a str,b:&'a str)->&'a str{match self{L::Ru=>a,L::En=>b}}}
const M:usize=4;
const J:[&str;8]=["/proc","/sys","/dev","/run","/.git","/node_modules","/.cache/thumbnails","/snap"];

fn h(m:u64)->String{let u=["B","KB","MB","GB","TB"];let mut s=m as f64;let mut i=0;while s>=1024.0&&i<4{s/=1024.0;i+=1}format!("{:.2} {}",s,u[i])}
fn ps(s:&str)->Option<u64>{let s=s.trim().to_uppercase();let(a,b)=if let Some(v)=s.strip_suffix("GB"){(v,1073741824)}else if let Some(v)=s.strip_suffix("MB"){(v,1048576)}else if let Some(v)=s.strip_suffix("KB"){(v,1024)}else{(s.as_str(),1)};a.trim().parse::<u64>().ok().map(|n|n*b)}
fn ig(p:&Path)->bool{let s=p.to_string_lossy();J.iter().any(|d|s.contains(d))}

fn cd(d:&Path,o:&mut Vec<PathBuf>,e:&AtomicUsize){
 let r=match read_dir(d){Ok(x)=>x,Err(_)=>{e.fetch_add(1,Relaxed);return}};
 for n in r.flatten(){let p=n.path();let m=match symlink_metadata(&p){Ok(v)=>v,Err(_)=>{e.fetch_add(1,Relaxed);continue}};
  if m.file_type().is_symlink(){continue}if m.is_dir(){if!ig(&p){o.push(p.clone());cd(&p,o,e)}}}}
fn sd(d:&Path,t:&mpsc::Sender<Q>,e:&AtomicUsize){
 let r=match read_dir(d){Ok(x)=>x,Err(_)=>{e.fetch_add(1,Relaxed);return}};
 for n in r.flatten(){let p=n.path();let m=match symlink_metadata(&p){Ok(v)=>v,Err(_)=>{e.fetch_add(1,Relaxed);continue}};
  if m.file_type().is_symlink(){continue}if m.is_file(){let _=t.send(Q{p,z:m.len()});}}}
fn ar(r:&Path)->(Vec<Q>,usize){
 let e=AtomicUsize::new(0);let mut d=Vec::new();cd(r,&mut d,&e);if!ig(r){d.push(r.to_path_buf())}
 let n=thread::available_parallelism().map(|x|x.get()).unwrap_or(4).max(2);let(t,q)=mpsc::channel();let k=(d.len()+n-1)/n;
 thread::scope(|s|{for c in d.chunks(k.max(1)){let t=t.clone();let e=&e;s.spawn(move||{for x in c{sd(x,&t,e)}});}});drop(t);
 (q.into_iter().collect(),e.load(Relaxed))}

fn ip(m:&str)->String{print!("{}",m);io::stdout().flush().ok();let mut s=String::new();io::stdin().read_line(&mut s).ok();s.trim().to_string()}
fn cl()->L{println!("Выберите язык / Choose language:\n  1 — Русский\n  2 — English");loop{match ip("> ").as_str(){"1"|"р"|"R"|"ру"=>return L::Ru,"2"|"e"|"E"|"en"=>return L::En,_=>println!("Введите 1 или 2 / Enter 1 or 2")}}}
fn gr(l:L){println!("\n🦫  SpaceHound 1.0 — {}",l.x("умный поиск больших файлов","smart large-file finder"));
 println!("{}",l.x("Параллельное сканирование, фильтры, экспорт и безопасный бэкап.\nНичего не удаляется без подтверждения.","Parallel scan, filters, export and safe backup.\nNothing is deleted without confirmation."));
 println!("⚠️  {}",l.x("Программа сделана ТОЛЬКО для Linux (Pop!_OS, Ubuntu, Debian и др.).","This program is made ONLY for Linux (Pop!_OS, Ubuntu, Debian, etc.)."));}
fn jr(p:&Path,l:L)->Option<String>{let n=p.file_name()?.to_string_lossy().to_lowercase();let x=p.extension().and_then(|v|v.to_str()).unwrap_or("").to_lowercase();let f=p.to_string_lossy().to_lowercase();
 if f.contains("/.fr-")||n.starts_with(".com.google"){return Some(l.x("временный файл распаковки","temporary extraction file").into())}
 if matches!(x.as_str(),"tmp"|"temp"|"log"|"old"|"bak"|"crdownload"|"part"){return Some(l.x("временный/резервный файл","temporary/backup file").into())}
 if n.contains("(1)")||n.contains("(2)")||n.contains("(3)")||n.contains("копия")||n.contains("copy"){return Some(l.x("похоже на дубликат","looks like a duplicate").into())}
 if x=="deb"||x=="rpm"||x=="msi"||x=="dmg"{return Some(l.x("установщик (после установки не нужен)","installer (not needed after install)").into())}
 if x=="exe"&&(n.contains("igram")||n.contains("mod")||n.contains("hack")){return Some(l.x("сторонний модифицированный клиент — ВНИМАНИЕ","third-party modified client — WARNING").into())}None}
fn hm()->PathBuf{PathBuf::from(var("HOME").unwrap_or_else(|_|"/home/steve".into()))}
fn bd()->PathBuf{let mut p=hm();p.push(".spacehound-backup");p}
fn bf(p:&Path,l:L)->bool{let b=bd();if create_dir_all(&b).is_err(){println!("❌ {}",l.x("не удалось создать папку бэкапа","could not create backup dir"));return false}
 let n=p.file_name().unwrap_or_default().to_string_lossy().to_string();let mut d=b.join(&n);
 if d.exists(){let s=p.file_stem().unwrap_or_default().to_string_lossy().to_string();let x=p.extension().map(|v|format!(".{}",v.to_string_lossy())).unwrap_or_default();let mut i=1;loop{d=b.join(format!("{}_{}{}",s,i,x));if!d.exists(){break}i+=1}}
 match rename(p,&d){Ok(_)=>{println!("   ✅ {} → {}",l.x("перемещено в бэкап","moved to backup"),d.display());true},Err(e)=>{if copy(p,&d).is_ok(){remove_file(p).ok();println!("   ✅ {} → {}",l.x("скопировано в бэкап","copied to backup"),d.display());true}else{println!("   ❌ {}: {}",l.x("ошибка","error"),e);false}}}}
fn er(p:&Path,f:&[Q],l:L){let mut c=format!("SpaceHound 1.0 — {}\n",l.x("отчёт","report"));for(i,v)in f.iter().enumerate(){c.push_str(&format!("{}. {} — {}\n",i+1,v.p.display(),h(v.z)))}if write(p,c).is_ok(){println!("✅ {} → {}",l.x("Отчёт сохранён","Report saved"),p.display())}else{println!("❌ {}",l.x("не удалось сохранить отчёт","could not save report"))}}

fn so(l:L,a:&[String],o:Option<&str>,t:usize)->(bool,Option<String>,bool,bool){
 println!("\n{} {}/{}",l.x("Попытка сканирования:","Scan attempt:"),t,M);
 let r=if let Some(p)=o{p.to_string()}else if a.len()>=2&&!a[1].starts_with("--"){a[1].clone()}else{ip(&l.x("\nВведите путь для сканирования (Enter = весь дом): ","\nEnter path to scan (Enter = whole home): "))};
 let r=if r.is_empty(){hm().to_string_lossy().to_string()}else{r};
 let n=if let Some(i)=a.iter().position(|v|v=="-n"){a.get(i+1).and_then(|v|v.parse().ok()).unwrap_or(20)}else{20};
 let mn=if let Some(i)=a.iter().position(|v|v=="--min"){a.get(i+1).and_then(|v|ps(v)).unwrap_or(0)}else{0};
 let ef=if let Some(i)=a.iter().position(|v|v=="--ext"){a.get(i+1).cloned().unwrap_or_default().to_lowercase()}else{String::new()};
 let ep=if let Some(i)=a.iter().position(|v|v=="--export"){a.get(i+1).cloned()}else{None};
 let p=Path::new(&r);if!p.exists(){println!("❌ {}: {}",l.x("путь не найден","path not found"),r);return(true,Some(r),false,false)}
 println!("\n🔍 {} {} ...",l.x("Параллельное сканирование","Parallel scan"),r);let st=std::time::Instant::now();let(mut f,e)=ar(p);let el=st.elapsed();
 let tf=f.len();let tb:u64=f.iter().map(|v|v.z).sum();f.retain(|v|v.z>=mn);if!ef.is_empty(){f.retain(|v|v.p.extension().and_then(|v|v.to_str()).unwrap_or("").to_lowercase()==ef)}
 f.sort_by(|a,b|b.z.cmp(&a.z));let top:Vec<Q>=f.iter().take(n).cloned().collect();
 let mut es:std::collections::HashMap<String,u64>=std::collections::HashMap::new();for v in &f{let x=v.p.extension().and_then(|v|v.to_str()).unwrap_or("(нет)").to_lowercase();*es.entry(x).or_insert(0)+=v.z}
 let mut et:Vec<_>=es.into_iter().collect();et.sort_by(|a,b|b.1.cmp(&a.1));et.truncate(5);
 println!("\n✅ {}: {} {} {:.2?} ({} {})",l.x("Найдено","Found"),f.len(),l.x("файлов за","files in"),el,e,l.x("ошибок","errors"));
 println!("{}: {} | {}: {}",l.x("Всего","Total"),tf,l.x("Суммарно","Sum"),h(tb));println!("{}:",l.x("Топ расширений","Top extensions"));for(x,z)in &et{println!("   .{:<10} {}",x,h(*z))}
 println!("\n{:<5} {:<13} {:<45} {}","№",l.x("РАЗМЕР","SIZE"),l.x("ФАЙЛ","FILE"),l.x("КОММЕНТАРИЙ","NOTE"));
 let mut su=Vec::new();for(i,v)in top.iter().enumerate(){let rs=jr(&v.p,l);if rs.is_some(){su.push(v.clone())}
  println!("{:<5} {:<13} {:<45} {}",i+1,h(v.z),{let s=v.p.file_name().unwrap_or_default().to_string_lossy().to_string();if s.chars().count()>44{let q:String=s.chars().take(42).collect();format!("{}...",q)}else{s}},rs.clone().unwrap_or_default())}
 if let Some(ep)=ep{er(Path::new(&ep),&top,l)}
 if!su.is_empty(){println!("\n⚠️  {}:",l.x("Рекомендации","Recommendations"));for(i,v)in su.iter().enumerate(){println!("  [{}] {} — {}",i+1,v.p.display(),h(v.z))}
  println!("\n{}",l.x("Резервная копия: ~/.spacehound-backup (файл НЕ удаляется, а перемещается туда).","Backup: ~/.spacehound-backup (the file is MOVED there, not deleted)."));
  let q=ip(&l.x("\nВведите номера через запятую для переноса в бэкап (Enter = пропустить): ","\nEnter comma-separated numbers to move to backup (Enter = skip): "));
  for c in q.split(','){if let Ok(i)=c.trim().parse::<usize>(){if i>=1&&i<=su.len(){bf(&su[i-1].p,l);}}}}else{println!("\n{}",l.x("Подозрительных файлов не найдено. Всё чисто!","No suspicious files found. All clean!"))}
 if t>=M{println!("\n{}",l.x("Сканирование окончательно завершено, повторите еще раз spacehound чтобы просканировать еще раз.","Scanning is finally complete, run spacehound again to scan once more."));return(false,Some(r),false,false)}
 println!("\n{}",l.x("[Вернуться] — начать сначала","[Back] — start over"));
 let q=ip(&l.x("\nПросканировать ещё раз? (Enter=выход, 1=тот же путь, 2=другой путь, 3=вернуться): ","\nScan again? (Enter=exit, 1=same path, 2=different path, 3=back): "));
 match q.as_str(){"1"=>(true,Some(r),true,false),"2"=>(true,Some(r),false,false),"3"=>{gr(l);(true,None,false,true)},_=>(false,Some(r),false,false)}}

fn main(){
 let d=std::panic::take_hook();std::panic::set_hook(Box::new(move|i|{eprintln!("\n⚠️  SpaceHound: {}",i);d(i)}));
 let l=if args().len()==1{gr(L::Ru);let x=cl();gr(x);x}else if args().any(|v|v=="--ru"){L::Ru}else{L::En};
 let a:Vec<String>=args().collect();if a.iter().any(|v|v=="--help"||v=="-h"){
  println!("\n{}",l.x("Использование:\n  spacehound                    интерактивный режим\n  spacehound ПУТЬ [-n N]         сканировать папку\n  spacehound ПУТЬ --min 100M     файлы от 100 МБ\n  spacehound ПУТЬ --ext mp4      только .mp4\n  spacehound ПУТЬ --export out.txt  сохранить отчёт\n  spacehound ПУТЬ --ru           русский вывод","Usage:\n  spacehound                    interactive mode\n  spacehound PATH [-n N]         scan a folder\n  spacehound PATH --min 100M     files from 100 MB\n  spacehound PATH --ext mp4      only .mp4\n  spacehound PATH --export out.txt  save report\n  spacehound PATH --ru           Russian output"));return}
 let mut rm:Option<String>=None;let mut rs=false;let mut t=1;
 loop{let o:Option<&str>=if rs{rm.as_deref()}else{None};let(c,v,s,z)=so(l,&a,o,t);if let Some(p)=v{rm=Some(p)}if!c{break}rs=s;if z{t=1}else{t+=1}}}
