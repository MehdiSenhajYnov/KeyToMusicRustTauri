import { chromium } from 'playwright-core';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
const azerty = process.env.KTM_TEST_AZERTY === '1';
const artifacts = process.env.KTM_SMOKE_ARTIFACTS || join(tmpdir(), 'keytomusic-ui-smoke');
await mkdir(artifacts, { recursive: true });
const server = spawn(process.execPath, ['node_modules/vite/bin/vite.js', '--host', '127.0.0.1', '--port', '5199', '--strictPort'], { stdio: 'pipe' });
server.stderr.on('data', chunk => process.stderr.write(chunk));
server.on('error', error => { throw error; });
let ready = false;
for (let attempt = 0; attempt < 100; attempt++) {
  if (server.exitCode !== null) throw Error('UI test server exited');
  try { if ((await fetch('http://127.0.0.1:5199/')).ok) { ready = true; break; } } catch {}
  await new Promise(resolve => setTimeout(resolve, 100));
}
if (!ready) { server.kill(); throw Error('UI test server did not start'); }
process.on('exit', () => server.kill());
const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH, headless: true, args: ['--no-sandbox'] });
const page = await browser.newPage({ viewport: { width: 1200, height: 800 } });
const errors = [];
page.on('pageerror', error => errors.push(error.message));
await page.addInitScript((azerty) => {
  if (azerty) Object.defineProperty(navigator, "keyboard", {value:undefined, configurable:true});
  const callbacks = new Map(); const listeners = new Map(); let next = 0;
  const profile = { id: 'a', name: 'Audit Manga', createdAt: '', updatedAt: '', tracks: [{id:'t1',name:'OST',volume:0.25},{id:'t2',name:'Ambiance',volume:0.5}], sounds: [
    {id:'s1',name:'Opening',momentum:5,duration:120,volume:0.7,source:{type:'local',path:'/tmp/opening.wav'}},
    {id:'s2',name:'Rain',momentum:0,duration:60,volume:0.5,source:{type:'local',path:'/tmp/rain.wav'}}], keyBindings:[
    {keyCode:'KeyA',trackId:'t1',soundIds:['s1'],loopMode:'single',currentIndex:0,mood:'epic'},
    {keyCode:'KeyA',trackId:'t2',soundIds:['s2'],loopMode:'sequential',currentIndex:0,mood:'peaceful'}]};
  if (azerty) profile.keyBindings.push({keyCode:'KeyQ',trackId:'t1',soundIds:['s2'],loopMode:'off',currentIndex:0});
  const config = {masterVolume:0.8,autoMomentum:false,keyDetectionEnabled:true,stopAllShortcut:['ControlLeft','ShiftLeft','KeyS'],autoMomentumShortcut:[],keyDetectionShortcut:[],crossfadeDuration:500,keyCooldown:200,currentProfileId:'a',audioDevice:null,chordWindowMs:30,momentumModifier:'Shift',playlistImportEnabled:false,moodAiEnabled:false,moodApiPort:8765};
  window.__audit = { commands: [], profiles: { a:profile, b:{...structuredClone(profile),id:'b',name:'Other profile'} }, config, failSave:false };
  const emit = (event,payload) => (listeners.get(event)||[]).forEach(id => callbacks.get(id)?.({event,payload}));
  window.__audit.emit = emit;
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {unregisterListener:(_event,id)=>callbacks.delete(id)};
  window.__TAURI_INTERNALS__ = {
    metadata: {currentWindow:{label:'main'},currentWebview:{label:'main',windowLabel:'main'}},
    transformCallback: cb => { callbacks.set(++next,cb); return next; },
    unregisterCallback: id => callbacks.delete(id),
    convertFileSrc: path => path,
    invoke: async (cmd,args={}) => {
      window.__audit.commands.push({cmd,args:structuredClone(args)});
      const audit = window.__audit;
      if (cmd === 'plugin:event|listen') {listeners.set(args.event,[...(listeners.get(args.event)||[]),args.handler]); return args.handler;}
      if (cmd === 'plugin:event|unlisten') {listeners.set(args.event,(listeners.get(args.event)||[]).filter(id=>id!==args.eventId));return;}
      if (cmd === 'get_keyboard_layout') return azerty ? {KeyA:'q',KeyQ:'a',KeyW:'z',KeyZ:'w',Digit1:'&'} : {};
      if (cmd === 'get_initial_state') return {config:audit.config,profiles:Object.values(audit.profiles),currentProfile:structuredClone(audit.profiles[audit.config.currentProfileId]),inputRuntime:{isLinux:false,isWayland:false,browserKeyFallback:true}};
      if (cmd === 'load_profile') return structuredClone(audit.profiles[args.id]);
      if (cmd === 'list_profiles') return Object.values(audit.profiles);
      if (cmd === 'save_profile') {if(audit.failSave)throw Error('Disk full (test)');audit.profiles[args.profile.id]=structuredClone(args.profile);return;}
      if (cmd === 'delete_profile') {delete audit.profiles[args.id];return;}
      if (cmd === 'create_profile') {const p={id:'created'+Date.now(),name:args.name,createdAt:'',updatedAt:'',tracks:[],sounds:[],keyBindings:[]};audit.profiles[p.id]=p;return p;}
      if (cmd === 'update_config') {Object.assign(audit.config,args.updates);return;}
      if (cmd === 'get_config') return audit.config;
      if (cmd === 'get_waveforms_batch') return Object.fromEntries(args.entries.map(e=>[e.path,{points:Array.from({length:100},(_,i)=>Math.abs(Math.sin(i/5))),duration:120,suggestedMomentum:5}]));
      if (cmd === 'get_waveform') return {points:[0.1,0.5,0.8,0.3],duration:120};
      if (cmd === 'verify_profile_sounds' || cmd === 'list_audio_devices' || cmd === 'list_disliked_videos') return [];
      if (cmd === 'preload_profile_sounds') return {};
      if (cmd.startsWith('check_')) return false;
      if (cmd === 'get_mood_service_status') return {runtime:'stopped',api:'disabled',enabled:false,port:8765};
      if (cmd === 'get_discovery_suggestions') return null;
      if (cmd === 'start_discovery') return [];
      if (cmd === 'play_sound') {emit('sound_started',{trackId:args.trackId,soundId:args.soundId});return;}
      if (cmd === 'stop_all_sounds') {emit('stop_all_triggered',{});return;}
      if (cmd === 'pick_audio_files') return ['/tmp/test.wav'];
      if (cmd === 'get_audio_duration') return 10;
      return undefined;
    }
  };
}, azerty);
try {
  await page.goto('http://127.0.0.1:5199/');
  await page.getByText('Audit Manga',{exact:true}).waitFor();
  await page.waitForFunction(() => getComputedStyle(document.querySelector('#root')).opacity === '1');
  if (azerty) {
    assert.equal(await page.locator('main button[title="Q"]').count(),1);
    assert.equal(await page.locator('main button[title="A"]').count(),1);
    await page.reload();
    await page.getByText('Audit Manga',{exact:true}).waitFor();
    assert.equal(await page.locator('main button[title="Q"]').count(),1);
    assert.equal(await page.locator('main button[title="A"]').count(),1);
  }
  await page.screenshot({path:join(artifacts,'main.png')});
  await page.keyboard.press('a');
  await page.waitForFunction(()=>window.__audit.commands.filter(c=>c.cmd==='play_sound').length===2);
  const volumes=await page.evaluate(()=>window.__audit.commands.filter(c=>c.cmd==='play_sound').map(c=>c.args.trackVolume));
  assert.deepEqual(volumes,[0.25,0.5]);
  if (azerty) {
    assert.equal(await page.locator('main button[title="Q"]').count(),1);
    assert.equal(await page.locator('main button[title="A"]').count(),1);
    const bindings = await page.evaluate(()=>JSON.stringify(window.__audit.profiles.a.keyBindings));
    await page.evaluate(()=>window.__audit.emit('keyboard_layout_changed',{KeyA:'a',KeyQ:'q'}));
    await page.waitForFunction(()=>document.querySelector('main button[title="A"]')?.textContent.includes('Opening'));
    await page.evaluate(()=>window.__audit.emit('keyboard_layout_changed',{KeyA:'q',KeyQ:'a'}));
    await page.waitForFunction(()=>document.querySelector('main button[title="Q"]')?.textContent.includes('Opening'));
    assert.equal(await page.evaluate(()=>JSON.stringify(window.__audit.profiles.a.keyBindings)),bindings);
  }
  await page.getByTitle('Settings',{exact:true}).click();
  await page.getByRole('dialog',{name:'Settings',exact:true}).waitFor();
  await page.keyboard.press('a');
  assert.equal(await page.evaluate(()=>window.__audit.commands.filter(c=>c.cmd==='play_sound').length),2);
  await page.screenshot({path:join(artifacts,'settings.png')});
  await page.keyboard.press('Escape');
  await page.getByRole('dialog',{name:'Settings',exact:true}).waitFor({state:'hidden'});
  await page.getByTitle('Keyboard Shortcuts',{exact:true}).click();
  await page.getByRole('dialog',{name:'Keyboard shortcuts'}).waitFor();
  for(let i=0;i<12;i++) await page.keyboard.press('Tab');
  assert.equal(await page.evaluate(()=>document.querySelector('[role="dialog"]').contains(document.activeElement)),true);
  await page.keyboard.press('Escape');
  await page.getByRole('button',{name:'+ Add Sound',exact:true}).click();
  await page.getByRole('dialog',{name:'Add sounds'}).waitFor();
  await page.screenshot({path:join(artifacts,'add-sound.png')});
  await page.getByRole('button',{name:'Add Files',exact:true}).click();
  await page.getByTitle('Play from momentum',{exact:true}).click();
  await page.waitForFunction(()=>window.__audit.commands.filter(c=>c.cmd==='play_sound').length===3);
  assert((await page.evaluate(()=>window.__audit.commands.filter(c=>c.cmd==='play_sound').at(-1).args.trackId)).startsWith('preview-'));
  await page.keyboard.press('Escape');
  await page.getByRole('alertdialog').waitFor();
  await page.getByRole('alertdialog').getByRole('button',{name:'Cancel',exact:true}).click();
  await page.getByRole('dialog',{name:'Add sounds'}).waitFor();
  await page.keyboard.press('Escape');
  await page.getByRole('alertdialog').getByRole('button',{name:'Confirm',exact:true}).click();
  await page.getByRole('dialog',{name:'Add sounds'}).waitFor({state:'hidden'});
  const stopped=await page.evaluate(()=>window.__audit.commands.filter(c=>c.cmd==='stop_sound').map(c=>c.args.trackId));
  assert(stopped.every(id=>id.startsWith('preview-')));
  // Test visible save failure and recovery through the real store + UI.
  await page.evaluate(async()=>{window.__audit.failSave=true;const {useProfileStore}=await import('/src/stores/profileStore.ts');useProfileStore.getState().updateTrack('t1',{name:'Saved OST'});});
  await page.getByRole('button',{name:'Unsaved changes · Retry'}).waitFor();
  await page.evaluate(()=>{window.__audit.failSave=false;});
  await page.getByRole('button',{name:'Unsaved changes · Retry'}).click();
  await page.waitForFunction(()=>window.__audit.profiles.a.tracks[0].name==='Saved OST');
  await page.evaluate(async()=>{const {useToastStore}=await import('/src/stores/toastStore.ts');useToastStore.setState({toasts:[]});});
  await page.setViewportSize({width:800,height:600});
  await page.screenshot({path:join(artifacts,'minimum-size.png')});
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>window.innerWidth),false);
  await page.locator('main').getByRole('button',{name:/Opening/}).click();
  await page.getByRole('button',{name:'Ambiance',exact:true}).click();
  await page.getByText(`Sounds for key [${azerty ? 'Q' : 'A'}]`,{exact:true}).waitFor();
  await page.screenshot({path:join(artifacts,'multi-track-details.png')});
  await page.getByRole('button',{name:'+ Add Sound to Key',exact:true}).click();
  await page.getByRole('button',{name:'Add Files',exact:true}).click();
  await page.getByRole('dialog',{name:'Add sounds'}).getByRole('button',{name:'Add Sound',exact:true}).click();
  await page.getByRole('dialog',{name:'Add sounds'}).waitFor({state:'hidden'});
  assert.equal(await page.evaluate(async()=>{const {useProfileStore}=await import('/src/stores/profileStore.ts');return useProfileStore.getState().currentProfile.sounds.length;}),3);
  await page.keyboard.press('Control+z');
  assert.equal(await page.evaluate(async()=>{const {useProfileStore}=await import('/src/stores/profileStore.ts');return useProfileStore.getState().currentProfile.sounds.length;}),2);
  await page.getByRole('button',{name:'Close',exact:true}).click();
  // Deleting an inactive profile must leave the active profile selected.
  const other = page.getByRole('button',{name:'Other profile'});
  await other.getByTitle('Delete',{exact:true}).click();
  await page.getByRole('alertdialog').waitFor();
  await page.getByRole('button',{name:'Confirm',exact:true}).click();
  await page.waitForFunction(()=>!window.__audit.profiles.b);
  assert.equal(await page.evaluate(()=>window.__audit.config.currentProfileId),'a');
  console.log(JSON.stringify({status:'passed',scenarios:['startup','multi-track playback and saved volume','modal shortcut suspension','Escape and focus trap','preview isolation','discard draft confirmation','single-step undo after sound addition','save failure and retry','800x600 layout','multi-track detail navigation','inactive profile deletion',...(azerty ? ['AZERTY before first keypress and after reload','stable A/Q labels during playback','live layout changes preserve bindings'] : [])],errors},null,2));
  assert.deepEqual(errors,[]);
} finally {await browser.close(); server.kill();}
