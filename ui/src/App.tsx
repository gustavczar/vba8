import { useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import Instructions from './Instructions';
type Source = { path: string; inspection: { status?: string; can_recover?: boolean; [key: string]: unknown } };
export default function App() {
  const [source, setSource] = useState<Source | null>(null);
  const [authorized, setAuthorized] = useState(false);
  const [result, setResult] = useState<{ output: string; report: { verified: boolean; warnings?: string[] } } | null>(null);
  const [phase, setPhase] = useState<'empty' | 'inspecting' | 'ready' | 'recovering' | 'success' | 'error'>('empty');
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [help, setHelp] = useState(false);
  const labels: Record<string,string> = {protected:'Proteção VBA identificada',unprotected:'Projeto sem senha VBA',already_recovered:'Projeto já processado',missing_project:'PROJECT ausente',missing_vba:'Nenhum projeto VBA',unknown:'Proteção não reconhecida'};
  const lock = useRef(false);
  const busy = phase === 'inspecting' || phase === 'recovering';
  const ready = source?.inspection.can_recover === true;
  function fail(cause: unknown) {
    const message = cause instanceof Error ? cause.message : typeof cause === 'string' ? cause : 'Não foi possível concluir a operação.';
    setError(/invoke|__TAURI|ipc/i.test(message) ? 'Abra o aplicativo desktop VBA8 para selecionar e salvar arquivos. A versão no navegador é apenas uma visualização.' : message);
    setPhase('error');
  }
  async function recover() {
    if (!source || !authorized || !ready || lock.current) return;
    lock.current = true; setPhase('recovering'); setError(''); setNotice(''); setResult(null);
    try {
      const saved = await invoke<NonNullable<typeof result> | null>('recover_selected', { source: source.path });
      setResult(saved); setPhase(saved ? 'success' : 'ready');
      if (!saved) setNotice('Salvamento cancelado. Nenhuma cópia foi gerada.');
    } catch (cause) { fail(cause); } finally { lock.current = false; }
  }
  async function selectSource() {
    if (lock.current) return;
    lock.current = true; setPhase('inspecting'); setSource(null); setAuthorized(false); setResult(null); setError(''); setNotice('');
    try {
      const next = await invoke<Source | null>('select_source');
      setSource(next); setPhase(next ? 'ready' : 'empty');
    } catch (cause) { fail(cause); } finally { lock.current = false; }
  }
  return (
    <main>
      <header><div className="brand"><span className="brand-icon" aria-hidden="true">▦</span>VBA<span>8</span><small>RECOVERY CARTRIDGE</small></div><button className="secondary" onClick={()=>setHelp(true)}>Instruções <span aria-hidden="true">?</span></button></header>
      <div className="workspace"><section className="intro"><div className="eyebrow">UTILITÁRIO LOCAL / EXCEL .XLSM</div><h1>Seu código.<br/>De volta<br/><em>ao controle.</em></h1><p>Recupere o acesso ao projeto VBA de uma planilha sua. Uma nova cópia, sem tocar no original.</p><div className="cartridge" aria-hidden="true"><div className="cart-top"><span>VBA</span><b>8</b><div className="pixel-key">⌁</div></div><div className="cart-label">PROJECT / RECOVERY<br/><span>LOCAL · SEM UPLOAD</span></div><div className="contacts">▥▥▥▥▥▥</div></div><div className="intro-foot"><span className="led"/>Sem envio de arquivos. Sem execução de macros.</div></section>
      <section className="console" aria-label="Recuperação de projeto VBA"><div className="console-head"><span>ESTAÇÃO DE RECUPERAÇÃO</span><span>01 / XLSM</span></div><div className="panel-body"><div className="step"><span>01</span><div><h2>Insira sua planilha</h2><p>Selecione um arquivo .xlsm com projeto VBA.</p></div></div><div className={'file-slot '+(source?'loaded':'')}><div className="file-icon" aria-hidden="true">▤</div>{source?<><strong>{source.path.split(/[\\/]/).pop()}</strong><p className="path" title={source.path}>{source.path}</p><span className="status-text">{labels[source.inspection.status||'unknown']||labels.unknown}</span></>:<><strong>{phase==='inspecting'?'Inspecionando arquivo…':'Nenhum cartucho inserido'}</strong><p>O original permanece somente leitura.</p></>}<button className="secondary" disabled={busy} onClick={selectSource}>Selecionar arquivo <span aria-hidden="true">↗</span></button></div><div className="step"><span>02</span><div><h2>Gere uma nova cópia</h2><p>Você escolhe o nome e onde salvar.</p></div></div><label className="authorization"><input type="checkbox" disabled={busy} checked={authorized} onChange={e=>setAuthorized(e.target.checked)}/><span>Este arquivo é meu ou tenho autorização para recuperá-lo.</span></label><button className="primary generate" onClick={recover} disabled={busy||!ready||!authorized}>{phase==='recovering'?'Gravando e verificando…':'Gerar cópia'}<span aria-hidden="true">→</span></button><div className="feedback" aria-live="polite">{error&&<div className="error" role="alert">{error}</div>}{notice&&<p>{notice}</p>}{result&&<section className="success"><h3>{result.report.verified?'Cópia gerada e verificada':'Cópia gerada'}</h3><p>{result.output}</p><p>Validação no Excel pendente.</p><button className="text-button" onClick={()=>setHelp(true)}>Ver próximos passos →</button></section>}</div><div className="scope"><span>ESCOPO</span><p>Proteção de visualização do VBA. Não remove senha de abertura, proteção de células ou estrutura. Arquivos assinados são recusados por segurança.</p></div></div></section></div><footer><span>VBA8 / RECUPERAÇÃO AUTORIZADA</span><span>ORIGINAL INTACTO · CÓPIA VERIFICADA</span></footer>{help&&<Instructions close={()=>setHelp(false)}/>}
    </main>
  );
}
