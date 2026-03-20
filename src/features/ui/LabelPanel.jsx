import { useState } from 'react';

const T={
  bg0:'#07070f',bg1:'#0c0c1a',bg2:'#111122',line:'#1e1e35',
  dim:'#444466',muted:'#6666aa',text:'#c0c0e0',
  red:'#ff3355',cyan:'#00ccff',grn:'#00dd88',amb:'#ffaa00',purp:'#9966ff',
};

let C;
let I;
let P;

export const LABEL_COLORS=C=[
  {id:'red',color:'#ff3355',name:'Красный'},
  {id:'orange',color:'#ff8800',name:'Оранжевый'},
  {id:'yellow',color:'#ffdd00',name:'Жёлтый'},
  {id:'green',color:'#00dd88',name:'Зелёный'},
  {id:'cyan',color:'#00ccff',name:'Голубой'},
  {id:'blue',color:'#4488ff',name:'Синий'},
  {id:'purple',color:'#9966ff',name:'Фиолетовый'},
  {id:'pink',color:'#ff55aa',name:'Розовый'},
];

export const LABEL_ICONS=I=[
  {id:'camera',icon:'📷'},{id:'warning',icon:'⚠️'},{id:'target',icon:'🎯'},
  {id:'lock',icon:'🔒'},{id:'unlock',icon:'🔓'},{id:'star',icon:'⭐'},
  {id:'flag',icon:'🚩'},{id:'eye',icon:'👁'},{id:'danger',icon:'☢'},
  {id:'hack',icon:'💀'},{id:'ok',icon:'✅'},{id:'bug',icon:'🐛'},
];

export const PRIORITIES=P=[
  {id:'critical',label:'Критично',color:'#ff3355'},
  {id:'high',label:'Высокий',color:'#ff8800'},
  {id:'medium',label:'Средний',color:'#ffdd00'},
  {id:'low',label:'Низкий',color:'#00dd88'},
  {id:'info',label:'Инфо',color:'#4488ff'},
];


const inp={width:'100%',padding:'6px 9px',background:T.bg0,color:T.text,
  border:'1px solid '+T.line,borderRadius:'4px',fontSize:'12px',marginBottom:'6px',
  boxSizing:'border-box',fontFamily:'inherit',outline:'none'};

const btnFull=(color)=>({width:'100%',padding:'7px',background:color+'18',color,
  border:'1px solid '+color+'55',borderRadius:'4px',fontSize:'12px',
  fontWeight:700,cursor:'pointer',marginBottom:'6px',fontFamily:'inherit'});

const DFORM={name:'',description:'',address:'',lat:'',lng:'',
  color:'red',icon:'camera',priority:'medium',tags:'',notes:''};

export default function LabelPanel({labels=[],setLabels,onLabelClick}){
  const [creating,setCreating]=useState(false);
  const [filter,setFilter]=useState('all');
  const [search,setSearch]=useState('');
  const [editId,setEditId]=useState(null);
  const [form,setForm]=useState(DFORM);
  const upd=(k,v)=>setForm(p=>({...p,[k]:v}));

  const save=()=>{
    if(!form.name.trim())return alert('Введите название');
    const existing=editId?labels.find(l=>l.id===editId):null;
    const label={
      id:editId||'lbl_'+Date.now(),...form,
      lat:form.lat===''?null:parseFloat(form.lat),lng:form.lng===''?null:parseFloat(form.lng),
      tags:form.tags.split(',').map(s=>s.trim()).filter(Boolean),
      createdAt:existing?.createdAt||new Date().toISOString(),updatedAt:new Date().toISOString(),
    };
    if(editId){setLabels(p=>p.map(l=>l.id===editId?label:l));setEditId(null);}
    else setLabels(p=>[...p,label]);
    setForm(DFORM);setCreating(false);
  };

  const remove=(id)=>{if(confirm('Удалить метку?'))setLabels(p=>p.filter(l=>l.id!==id));};

  const startEdit=(label)=>{
    setForm({...label,tags:label.tags?.join(', ')||'',lat:label.lat??'',lng:label.lng??''});
    setEditId(label.id);setCreating(true);
  };

  const colorDef=(id)=>C.find(c=>c.id===id)||C[0];
  const iconDef=(id)=>I.find(c=>c.id===id)||I[0];
  const priDef=(id)=>P.find(p=>p.id===id)||P[2];

  const filtered=labels
    .filter(l=>filter==='all'||l.priority===filter)
    .filter(l=>!search||l.name.toLowerCase().includes(search.toLowerCase())||l.address?.toLowerCase().includes(search.toLowerCase()));

  if(creating)return(
    <div style={{background:T.bg2,border:'1px solid '+T.line,borderRadius:'6px',padding:'12px'}}>
      <div style={{display:'flex',justifyContent:'space-between',alignItems:'center',marginBottom:'12px'}}>
        <span style={{fontSize:'12px',fontWeight:700,color:T.text}}>{editId?'✏ Редактировать':'+ Новая метка'}</span>
        <button onClick={()=>{setCreating(false);setEditId(null);setForm(DFORM);}}
          style={{background:'none',border:'none',color:T.red,cursor:'pointer',fontSize:'14px'}}>×</button>
      </div>
      <div style={{fontSize:'10px',color:T.muted,marginBottom:'3px'}}>Название *</div>
      <input style={inp} value={form.name} onChange={e=>upd('name',e.target.value)} placeholder='Название метки' />
      <div style={{fontSize:'10px',color:T.muted,marginBottom:'3px'}}>Описание</div>
      <input style={inp} value={form.description} onChange={e=>upd('description',e.target.value)} placeholder='Краткое описание' />
      <div style={{fontSize:'10px',color:T.muted,marginBottom:'3px'}}>Адрес</div>
      <input style={inp} value={form.address} onChange={e=>upd('address',e.target.value)} placeholder='ул. Ленина, д. 5' />
      <div style={{display:'flex',gap:'6px',marginBottom:'6px'}}>
        <div style={{flex:1}}>
          <div style={{fontSize:'10px',color:T.muted,marginBottom:'3px'}}>Широта</div>
          <input style={{...inp,marginBottom:0}} value={form.lat} onChange={e=>upd('lat',e.target.value)} placeholder='53.9000' />
        </div>
        <div style={{flex:1}}>
          <div style={{fontSize:'10px',color:T.muted,marginBottom:'3px'}}>Долгота</div>
          <input style={{...inp,marginBottom:0}} value={form.lng} onChange={e=>upd('lng',e.target.value)} placeholder='27.5600' />
        </div>
      </div>
      <div style={{height:'6px'}}/>
      <div style={{fontSize:'10px',color:T.muted,marginBottom:'6px'}}>Иконка</div>
      <div style={{display:'flex',flexWrap:'wrap',gap:'4px',marginBottom:'10px'}}>
        {I.map(ic=>(
          <button key={ic.id} onClick={()=>upd('icon',ic.id)} title={ic.id}
            style={{width:'32px',height:'32px',fontSize:'16px',cursor:'pointer',
              background:form.icon===ic.id?T.amb+'30':T.bg0,
              border:'1px solid '+(form.icon===ic.id?T.amb:T.line),
              borderRadius:'5px',display:'flex',alignItems:'center',justifyContent:'center'}}>
            {ic.icon}</button>
        ))}
      </div>
      <div style={{fontSize:'10px',color:T.muted,marginBottom:'6px'}}>Цвет маркера</div>
      <div style={{display:'flex',gap:'5px',marginBottom:'10px',flexWrap:'wrap'}}>
        {C.map(c=>(
          <button key={c.id} onClick={()=>upd('color',c.id)} title={c.name}
            style={{width:'24px',height:'24px',borderRadius:'50%',cursor:'pointer',
              background:c.color,
              border:form.color===c.id?'3px solid white':'2px solid '+c.color+'80',
              boxShadow:form.color===c.id?'0 0 6px '+c.color:'none'}}/>
        ))}
      </div>
      <div style={{fontSize:'10px',color:T.muted,marginBottom:'6px'}}>Приоритет</div>
      <div style={{display:'flex',gap:'4px',marginBottom:'10px',flexWrap:'wrap'}}>
        {P.map(pr=>(
          <button key={pr.id} onClick={()=>upd('priority',pr.id)}
            style={{padding:'3px 8px',fontSize:'10px',cursor:'pointer',
              background:form.priority===pr.id?pr.color+'25':'transparent',
              color:form.priority===pr.id?pr.color:T.dim,
              border:'1px solid '+(form.priority===pr.id?pr.color+'60':T.line),
              borderRadius:'10px',fontFamily:'inherit'}}>
            {pr.label}</button>
        ))}
      </div>
      <div style={{fontSize:'10px',color:T.muted,marginBottom:'3px'}}>Теги (через запятую)</div>
      <input style={inp} value={form.tags} onChange={e=>upd('tags',e.target.value)} placeholder='cctv, hikvision' />
      <div style={{fontSize:'10px',color:T.muted,marginBottom:'3px'}}>Заметки оператора</div>
      <textarea style={{...inp,height:'60px',resize:'vertical'}} value={form.notes} onChange={e=>upd('notes',e.target.value)} placeholder='Дополнительная информация...' />
      <div style={{display:'flex',gap:'6px'}}>
        <button style={{...btnFull(T.amb),flex:2,marginBottom:0}} onClick={save}>{editId?'💾 Сохранить':'+ Создать метку'}</button>
        <button style={{...btnFull(T.muted),flex:1,marginBottom:0}} onClick={()=>{setCreating(false);setEditId(null);setForm(DFORM);}}>Отмена</button>
      </div>
    </div>
  );

  return(
    <div>
      <button style={btnFull(T.amb)} onClick={()=>{setCreating(true);setEditId(null);}}>+ Создать метку</button>
      <input style={inp} value={search} onChange={e=>setSearch(e.target.value)} placeholder='🔍 Поиск меток...' />
      <div style={{display:'flex',gap:'3px',flexWrap:'wrap',marginBottom:'10px'}}>
        {[{id:'all',label:'Все',color:T.muted},...P].map(pr=>(
          <button key={pr.id} onClick={()=>setFilter(pr.id)}
            style={{padding:'3px 8px',fontSize:'10px',cursor:'pointer',
              background:filter===pr.id?pr.color+'25':'transparent',
              color:filter===pr.id?pr.color:T.dim,
              border:'1px solid '+(filter===pr.id?pr.color+'60':T.line),
              borderRadius:'10px',fontFamily:'inherit'}}>
            {pr.label}</button>
        ))}
      </div>
      {filtered.length===0&&(
        <div style={{textAlign:'center',padding:'20px 0',color:T.dim,fontSize:'12px'}}>
          {labels.length===0?'Нет меток. Создайте первую →':'Нет совпадений'}
        </div>
      )}
      {filtered.map(label=>(
        <LabelCard key={label.id} label={label}
          colorDef={colorDef} iconDef={iconDef} priDef={priDef}
          onEdit={()=>startEdit(label)} onDelete={()=>remove(label.id)}
          onClick={()=>onLabelClick?.(label)}/>
      ))}
    </div>
  );
}

export function LabelCard({label,colorDef,iconDef,priDef,onEdit,onDelete,onClick}){
  const [open,setOpen]=useState(false);
  const col=colorDef(label.color);
  const ic=iconDef(label.icon);
  const pri=priDef(label.priority);
  return(
    <div style={{background:'#0c0c1a',border:'1px solid '+(open?col.color+'50':'#1e1e35'),
      borderLeft:'3px solid '+col.color,borderRadius:'5px',marginBottom:'5px',overflow:'hidden'}}>
      <div style={{display:'flex',alignItems:'center',gap:'8px',padding:'7px 10px',cursor:'pointer'}}
        onClick={()=>setOpen(v=>!v)}>
        <span style={{fontSize:'15px',flexShrink:0}}>{ic.icon}</span>
        <div style={{flex:1,minWidth:0}}>
          <div style={{fontSize:'12px',fontWeight:600,color:'#c0c0e0',whiteSpace:'nowrap',overflow:'hidden',textOverflow:'ellipsis'}}>{label.name}</div>
          {label.address&&<div style={{fontSize:'10px',color:'#6666aa',whiteSpace:'nowrap',overflow:'hidden',textOverflow:'ellipsis'}}>📍 {label.address}</div>}
        </div>
        <span style={{fontSize:'9px',padding:'2px 6px',borderRadius:'8px',
          background:pri.color+'20',color:pri.color,border:'1px solid '+pri.color+'30',flexShrink:0}}>{pri.label}</span>
        <span style={{color:'#444466',fontSize:'10px'}}>{open?'▲':'▼'}</span>
      </div>
      {open&&(
        <div style={{padding:'0 10px 10px',borderTop:'1px solid #1e1e35'}}>
          {label.description&&<div style={{fontSize:'11px',color:'#8888aa',margin:'8px 0'}}>{label.description}</div>}
          {label.lat!=null&&label.lng!=null&&(
            <div style={{fontSize:'10px',color:'#6666aa',marginBottom:'6px'}}>
              🌐 {Number(label.lat).toFixed(5)}, {Number(label.lng).toFixed(5)}{' '}
              <button onClick={()=>onClick?.(label)} style={{background:'none',border:'none',color:col.color,cursor:'pointer',fontSize:'10px',padding:0}}>→ на карте</button>
            </div>
          )}
          {label.tags?.length>0&&(
            <div style={{display:'flex',gap:'4px',flexWrap:'wrap',marginBottom:'6px'}}>
              {label.tags.map((tag,i)=>(
                <span key={i} style={{fontSize:'9px',background:'#111122',border:'1px solid #1e1e35',color:'#6666aa',padding:'2px 6px',borderRadius:'8px'}}>#{tag}</span>
              ))}
            </div>
          )}
          {label.notes&&(
            <div style={{fontSize:'10px',color:'#888899',background:'#07070f',border:'1px solid #1e1e35',padding:'6px 8px',borderRadius:'4px',marginBottom:'8px',whiteSpace:'pre-wrap'}}>{label.notes}</div>
          )}
          {label.createdAt&&(
            <div style={{fontSize:'9px',color:'#444466',marginBottom:'8px'}}>Создано: {new Date(label.createdAt).toLocaleString('ru')}</div>
          )}
          <div style={{display:'flex',gap:'6px'}}>
            <button onClick={onEdit} style={{flex:1,padding:'5px',background:'#111122',color:'#9966ff',border:'1px solid #9966ff40',borderRadius:'4px',fontSize:'10px',cursor:'pointer',fontFamily:'inherit'}}>✏ Изменить</button>
            <button onClick={onDelete} style={{flex:1,padding:'5px',background:'#120808',color:'#ff3355',border:'1px solid #ff335540',borderRadius:'4px',fontSize:'10px',cursor:'pointer',fontFamily:'inherit'}}>🗑 Удалить</button>
            {label.lat!=null&&label.lng!=null&&(
              <button onClick={()=>onClick?.(label)} style={{flex:1,padding:'5px',background:'#0a1520',color:col.color,border:'1px solid '+col.color+'40',borderRadius:'4px',fontSize:'10px',cursor:'pointer',fontFamily:'inherit'}}>📍 Карта</button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
