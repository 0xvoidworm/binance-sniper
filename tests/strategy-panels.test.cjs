const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

// Minimal DOM fixture: exercise real panel render/caching, with no dependency.
class Element {
  constructor(tag) { this.tagName=tag; this.children=[]; this.className=''; this.scrollTop=0; }
  get firstElementChild() { return this.children[0]; }
  get classList() { return {contains: name => this.className.split(' ').includes(name)}; }
  append(...nodes) { this.children.push(...nodes); }
  replaceChildren(...nodes) { this.children=[...nodes]; }
  querySelector(tag) { return this.children.find(e => e.tagName===tag) || null; }
}

function panels() {
  const context=vm.createContext({window:{},Node:Element,navigator:{onLine:true},Date,Map,
    document:{createElement:tag=>new Element(tag),createTextNode:text=>({textContent:text})}});
  for(const file of ['research-panel.js','lab-panel.js']) {
    vm.runInContext(fs.readFileSync(path.join(__dirname,'../web/assets',file),'utf8'),context);
  }
  return context.window;
}

test('cached phase 2 cannot leave Strategy Lab content in the Research tab', () => {
  const {ResearchPanel,LabPanel}=panels(),host=new Element('div');
  ResearchPanel.render(host,null);
  assert.ok(host.firstElementChild.classList.contains('phase2-toolbar'));
  LabPanel.render(host,null);
  assert.ok(host.firstElementChild.classList.contains('lab-toolbar'));
  ResearchPanel.render(host,null); // identical cached payload, but a different tab
  assert.ok(host.firstElementChild.classList.contains('phase2-toolbar'));
  assert.ok(!host.firstElementChild.classList.contains('lab-toolbar'));
});

test('cached Lab tab is restored after Research and stable renders keep its DOM', () => {
  const {ResearchPanel,LabPanel}=panels(),host=new Element('div');
  LabPanel.render(host,null);
  const original=host.firstElementChild;
  LabPanel.render(host,null);
  assert.equal(host.firstElementChild,original);
  ResearchPanel.render(host,null);
  LabPanel.render(host,null);
  assert.ok(host.firstElementChild.classList.contains('lab-toolbar'));
});
