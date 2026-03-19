import L from 'leaflet';
import { MapContainer, TileLayer, Marker, Popup, useMap } from 'react-leaflet';
import { useEffect } from 'react';
import icon from 'leaflet/dist/images/marker-icon.png';
import iconShadow from 'leaflet/dist/images/marker-shadow.png';

const DefaultIcon=L.icon({iconUrl:icon,shadowUrl:iconShadow,iconSize:[25,41],iconAnchor:[12,41]});
L.Marker.prototype.options.icon=DefaultIcon;

const C={red:'#ff3355',orange:'#ff8800',yellow:'#ffdd00',green:'#00dd88',cyan:'#00ccff',blue:'#4488ff',purple:'#9966ff',pink:'#ff55aa'}; // LABEL_COLORS_MAP
const LABEL_ICONS_MAP={camera:'📷',warning:'⚠',target:'🎯',lock:'🔒',unlock:'🔓',star:'★',flag:'⚑',eye:'👁',danger:'☢',hack:'☠',ok:'✓',bug:'⚡'};
const PRI_COLORS={critical:'#ff3355',high:'#ff8800',medium:'#ffdd00',low:'#00dd88',info:'#4488ff'};
const PRI_LABELS={critical:'Критично',high:'Высокий',medium:'Средний',low:'Низкий',info:'Инфо'};

const makeLabelIcon=function createLabelIcon(color,iconChar){
  const svg=`<svg xmlns='http://www.w3.org/2000/svg' width='32' height='40' viewBox='0 0 32 40'>
    <filter id='sh'><feDropShadow dx='0' dy='2' stdDeviation='2' flood-color='#000' flood-opacity='.4'/></filter>
    <path d='M16 0C8.27 0 2 6.27 2 14c0 10 14 26 14 26S30 24 30 14C30 6.27 23.73 0 16 0z'
      fill='${color}' filter='url(#sh)' stroke='rgba(255,255,255,0.3)' stroke-width='1'/>
    <circle cx='16' cy='14' r='8' fill='rgba(0,0,0,0.25)'/>
    <text x='16' y='19' text-anchor='middle' font-size='11' fill='white'>${iconChar}</text>
  </svg>`;
  return L.divIcon({html:svg,className:'',iconSize:[32,40],iconAnchor:[16,40],popupAnchor:[0,-40]});
};

const POPUP_CSS=`
  .hp .leaflet-popup-content-wrapper{background:#0c0c1a;border:1px solid #2a2a4a;border-radius:8px;padding:0;box-shadow:0 4px 24px rgba(0,0,0,.7);min-width:220px;max-width:280px;}
  .hp .leaflet-popup-content{margin:0;color:#c0c0e0;}
  .hp .leaflet-popup-tip{background:#0c0c1a;}
  .hp .leaflet-popup-close-button{color:#666;top:8px;right:8px;font-size:16px;}
  .hp-head{background:#111126;padding:10px 14px;border-bottom:1px solid #1e1e35;border-radius:8px 8px 0 0;}
  .hp-body{padding:8px 14px 12px;}
  .hp-btn{display:block;width:100%;padding:6px 10px;margin-bottom:4px;cursor:pointer;font-size:11px;font-weight:600;border-radius:4px;text-align:left;font-family:inherit;}
`;

function injectStyles(){
  if(document.getElementById('hp-styles'))return;
  const el=document.createElement('style');el.id='hp-styles';el.textContent=POPUP_CSS;
  document.head.appendChild(el);
}

function MapController({mapCenter}){
  const map=useMap();
  useEffect(()=>{if(mapCenter)map.setView(mapCenter,map.getZoom());},[mapCenter,map]);
  return null;
}

export default function StreamGrid({
  mapCenter,groupedMapTargets,onCameraPlayClick,
  setNemesisTarget,handleLocalArchive,handleFetchNvrDeviceInfo,
  handleFetchOnvifDeviceInfo,fetchFtpRoot,
  labels=[],onLabelClick:onLabelOpen,
}){
  useEffect(()=>{injectStyles();},[]);

  return(
    <MapContainer center={mapCenter||[53.9,27.56]} zoom={12}
      style={{width:'100%',height:'100%'}} zoomControl={true}>
      <TileLayer url='https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png' attribution='© OpenStreetMap'/>
      <MapController mapCenter={mapCenter}/>

      {Array.from(groupedMapTargets.values()).map(site=>(
        <Marker key={site.id} position={[site.lat,site.lng]}>
          <Popup className='hp' maxWidth={280}>
            <div>
              <div className={'hp'+'-head'}>
                <div style={{fontWeight:700,fontSize:'13px',color:'#fff',marginBottom:'3px'}}>📍 {site.siteName}</div>
                <div style={{fontSize:'10px',color:'#6666aa'}}>{site.terminals.length} терм. · {site.lat.toFixed(4)}, {site.lng.toFixed(4)}</div>
              </div>
              <div className='hp-body' style={{maxHeight:'320px',overflowY:'auto'}}>
                {site.terminals.map((t,ti)=>(
                  <div key={t.id} style={{marginTop:ti>0?'10px':0,paddingTop:ti>0?'10px':0,borderTop:ti>0?'1px solid #1e1e35':'none'}}>
                    <div style={{display:'flex',alignItems:'center',gap:'6px',marginBottom:'6px'}}>
                      <span style={{fontSize:'13px'}}>{t.type==='hub'?'🌐':'📷'}</span>
                      <div>
                        <div style={{fontSize:'12px',fontWeight:700,color:'#e0e0ff'}}>{t.name}</div>
                        <div style={{fontSize:'10px',color:'#6666aa',fontFamily:'monospace'}}>{t.host}</div>
                      </div>
                    </div>
                    {t.channels?.length>0&&(
                      <div style={{display:'flex',flexWrap:'wrap',gap:'4px',marginBottom:'8px'}}>
                        {t.channels.map(ch=>(
                          <button key={ch.id} onClick={()=>onCameraPlayClick?.({ip:t.host,terminalId:t.id,terminal:t,channel:ch})}
                            style={{padding:'4px 8px',cursor:'pointer',background:'#00ccff18',color:'#00ccff',border:'1px solid #00ccff40',borderRadius:'4px',fontSize:'10px',fontWeight:600,fontFamily:'inherit'}}>
                            ▶ {ch.name||`К-${(ch.index??0)+1}`}</button>
                        ))}
                      </div>
                    )}
                    {t.type==='hub'?(
                      <button className='hp-btn' onClick={()=>fetchFtpRoot?.('video1')}
                        style={{background:'#00ccff18',color:'#00ccff',border:'1px solid #00ccff40'}}>📁 Архив хаба (FTP)</button>
                    ):(
                      <>
                        <button className='hp-btn' onClick={()=>setNemesisTarget?.({host:t.host,login:t.login||'admin',password:t.password||'',name:t.name,channels:t.channels})}
                          style={{background:'#ff335518',color:'#ff3355',border:'1px solid #ff335540',letterSpacing:'.04em'}}>☢ Nemesis — взлом архива</button>
                        <button className='hp-btn' onClick={()=>handleLocalArchive?.(t)}
                          style={{background:'#ffaa0018',color:'#ffaa00',border:'1px solid #ffaa0040'}}>⏳ Запрос памяти NVR</button>
                        <div style={{display:'grid',gridTemplateColumns:'1fr 1fr',gap:'4px'}}>
                          <button className='hp-btn' onClick={()=>handleFetchNvrDeviceInfo?.(t)} style={{background:'#4488ff18',color:'#4488ff',border:'1px solid #4488ff40',marginBottom:0}}>ℹ ISAPI</button>
                          <button className='hp-btn' onClick={()=>handleFetchOnvifDeviceInfo?.(t)} style={{background:'#00dd8818',color:'#00dd88',border:'1px solid #00dd8840',marginBottom:0}}>ℹ ONVIF</button>
                        </div>
                      </>
                    )}
                  </div>
                ))}
              </div>
            </div>
          </Popup>
        </Marker>
      ))}

      {labels.filter(l=>l.lat&&l.lng).map(label=>{
        const color=C[label.color]||'#9966ff';
        const iconCh=LABEL_ICONS_MAP[label.icon]||'📍';
        const pri=PRI_COLORS[label.priority]||'#4488ff';
        const priLbl=PRI_LABELS[label.priority]||'Инфо';
        return(
          <Marker key={label.id} position={[label.lat,label.lng]} icon={makeLabelIcon(color,iconCh)}>
            <Popup className='hp' maxWidth={260}>
              <div>
                <div className={'hp'+'-head'} style={{borderLeft:'3px solid '+color}}>
                  <div style={{display:'flex',alignItems:'center',gap:'8px'}}>
                    <span style={{fontSize:'18px'}}>{iconCh}</span>
                    <div>
                      <div style={{fontWeight:700,fontSize:'13px',color:'#fff'}}>{label.name}</div>
                      {label.address&&<div style={{fontSize:'10px',color:'#6666aa'}}>📍 {label.address}</div>}
                    </div>
                  </div>
                </div>
                <div className='hp-body'>
                  <div style={{marginBottom:'8px'}}>
                    <span style={{fontSize:'10px',padding:'2px 8px',borderRadius:'10px',background:pri+'20',color:pri,border:'1px solid '+pri+'40',fontWeight:600}}>{priLbl}</span>
                  </div>
                  {label.description&&<div style={{fontSize:'11px',color:'#8888aa',marginBottom:'8px',lineHeight:1.5}}>{label.description}</div>}
                  {label.tags?.length>0&&<div style={{display:'flex',flexWrap:'wrap',gap:'3px',marginBottom:'8px'}}>
                    {label.tags.map((tag,i)=><span key={i} style={{fontSize:'9px',background:'#111122',border:'1px solid #1e1e35',color:'#6666aa',padding:'2px 6px',borderRadius:'8px'}}>#{tag}</span>)}
                  </div>}
                  {label.notes&&<div style={{fontSize:'10px',color:'#777788',background:'#07070f',border:'1px solid #1e1e35',padding:'6px 8px',borderRadius:'4px',marginBottom:'8px',whiteSpace:'pre-wrap'}}>{label.notes}</div>}
                  <div style={{fontSize:'10px',color:'#444466'}}>🌐 {label.lat.toFixed(5)}, {label.lng.toFixed(5)}</div>
                  <button className='hp-btn' onClick={()=>onLabelOpen?.(label)} style={{background:color+'18',color,border:'1px solid '+color+'40',marginTop:'8px'}}>✏ Открыть в панели меток</button>
                </div>
              </div>
            </Popup>
          </Marker>
        );
      })}
    </MapContainer>
  );
}
