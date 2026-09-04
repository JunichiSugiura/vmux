pub(crate) const WRY_HOST_SHIM: &str = r#"
(function () {
  const report = (kind, text) => {
    try { window.ipc.postMessage('log:' + kind + ':' + text); } catch (e) {}
  };
  const encoder = new TextEncoder();
  const nativeOpen = XMLHttpRequest.prototype.open;
  const nativeSend = XMLHttpRequest.prototype.send;
  XMLHttpRequest.prototype.open = function (method, url, ...rest) {
    this.__vmuxEvent = String(url).endsWith('/__events');
    return nativeOpen.call(this, method, url, ...rest);
  };
  XMLHttpRequest.prototype.send = function (body) {
    if (this.__vmuxEvent) {
      const selected = !(document.getSelection() || { isCollapsed: true }).isCollapsed;
      this.setRequestHeader('x-vmux-selected', selected ? '1' : '0');
      const el = document.activeElement;
      if (el && typeof el.selectionStart === 'number' && /^[\w:.-]+$/.test(el.id)) {
        const bytes = (upto) => encoder.encode(el.value.slice(0, upto)).length;
        this.setRequestHeader(
          'x-vmux-caret',
          el.id + ':' + bytes(el.selectionStart) + ':' + bytes(el.selectionEnd),
        );
      }
    }
    return nativeSend.call(this, body);
  };
  window.addEventListener('error', (e) => {
    report('error', (e.message || 'error') + ' @ ' + (e.filename || '?') + ':' + (e.lineno || 0));
  });
  window.addEventListener('unhandledrejection', (e) => {
    report('reject', String((e.reason && e.reason.stack) || e.reason));
  });
  for (const level of ['error', 'warn', 'log']) {
    const original = console[level].bind(console);
    console[level] = (...args) => {
      report(level, args.map((a) => {
        if (a instanceof Error) return a.stack || a.message;
        if (typeof a === 'object') { try { return JSON.stringify(a); } catch (e) { return String(a); } }
        return String(a);
      }).join(' '));
      original(...args);
    };
  }
  const listeners = new Map();
  function toBase64(buffer) {
    const bytes = new Uint8Array(buffer);
    let binary = '';
    for (let i = 0; i < bytes.length; i += 1) binary += String.fromCharCode(bytes[i]);
    return btoa(binary);
  }
  function fromBase64(text) {
    const binary = atob(text);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
    return bytes.buffer;
  }
  const measureNode = (node, what) => {
    const i = window.interpreter;
    if (what === 'rect') {
      const rect = i.getClientRect(node);
      return rect ? [rect.origin[0], rect.origin[1], rect.size[0], rect.size[1]] : [];
    }
    const pair =
      what === 'scrollSize'
        ? [i.getScrollWidth(node), i.getScrollHeight(node)]
        : [i.getScrollLeft(node), i.getScrollTop(node)];

    return pair.some((n) => n === undefined) ? [] : [pair[0], pair[1], 0, 0];
  };
  let caretRuler = null;
  const scrollCaretIntoView = (el, index) => {
    if (el.scrollWidth <= el.clientWidth) return;
    const style = getComputedStyle(el);
    caretRuler = caretRuler || document.createElement('canvas');
    const pen = caretRuler.getContext('2d');
    pen.font = style.font;
    const caret = pen.measureText(el.value.slice(0, index)).width;
    const view =
      el.clientWidth - (parseFloat(style.paddingLeft) || 0) - (parseFloat(style.paddingRight) || 0);
    const margin = view / 4;
    if (caret < el.scrollLeft) el.scrollLeft = Math.max(0, caret - margin);
    else if (caret > el.scrollLeft + view) el.scrollLeft = caret - view + margin;
  };
  const textOffsetAtPoint = (element, x, y) => {
    const el = document.getElementById(element);
    if (!el) return [];
    let node = null;
    let offset = 0;
    if (document.caretPositionFromPoint) {
      const position = document.caretPositionFromPoint(x, y);
      if (position) { node = position.offsetNode; offset = position.offset; }
    } else if (document.caretRangeFromPoint) {
      const range = document.caretRangeFromPoint(x, y);
      if (range) { node = range.startContainer; offset = range.startOffset; }
    }
    if (node && el.contains(node)) {
      const range = document.createRange();
      range.selectNodeContents(el);
      range.setEnd(node, offset);
      return [[...range.cloneContents().textContent].length, 0, 0, 0];
    }
    const rect = el.getBoundingClientRect();
    if (rect.width <= 0) return [0, 0, 0, 0];
    const length = [...(el.textContent || '')].length;
    const ratio = Math.min(Math.max((x - rect.left) / rect.width, 0), 1);
    return [Math.round(ratio * length), 0, 0, 0];
  };
  const applyDomRequest = (request) => {
    switch (request.kind) {
      case 'focusNode':
        window.interpreter.setFocus(request.node, request.focus);
        return;
      case 'scrollNode':
        window.interpreter.scroll(request.node, request.x, request.y, request.behavior);
        return;
      case 'revealNode':
        window.interpreter.scrollTo(request.node, {
          behavior: request.behavior,
          block: request.block,
          inline: request.inline,
        });
        return;
      case 'measureNode':
        window.ipc.postMessage(
          'measured:' + request.token + ':' + measureNode(request.node, request.what).join(','),
        );
        return;
      case 'revealElement':
        for (const id of request.elements) {
          const target = document.getElementById(id);
          if (target) {
            target.scrollIntoView({ block: request.block, inline: 'nearest' });
            break;
          }
        }
        return;
      case 'textOffsetAtPoint':
        window.ipc.postMessage(
          'measured:' +
            request.token +
            ':' +
            textOffsetAtPoint(request.element, request.x, request.y).join(','),
        );
        return;
    }
    const el = document.getElementById(request.element);
    if (!el) return;
    switch (request.kind) {
      case 'focus':
        el.focus({ preventScroll: true });
        break;
      case 'showPopover':
        if (el.isConnected && !el.matches(':popover-open')) {
          try { el.showPopover(); } catch (e) {}
        }
        break;
      case 'clearText':
        el.value = '';
        break;
      case 'toggleMedia':
        if (el.paused) { el.play(); } else { el.pause(); }
        break;
      case 'scrollIntoView':
        el.scrollIntoView({ block: 'nearest', inline: 'nearest' });
        break;
      case 'scrollTo':
        el.scrollTo({ top: request.top, behavior: 'instant' });
        break;
      case 'selectAll':
        el.setSelectionRange(0, el.value.length);
        break;
      case 'offerText':
        requestAnimationFrame(() => {
          el.focus();
          el.setSelectionRange(0, el.value.length);
          el.scrollLeft = 0;
        });
        break;
      case 'placeCaret': {
        const bytes = new TextEncoder().encode(el.value).slice(0, request.byte);
        const index = new TextDecoder().decode(bytes).length;
        el.setSelectionRange(index, index);
        scrollCaretIntoView(el, index);
        break;
      }
      case 'caretToEnd':
        requestAnimationFrame(() => {
          const end = el.value.length;
          el.setSelectionRange(end, end);
          el.scrollLeft = el.scrollWidth;
          el.scrollTop = el.scrollHeight;
        });
        break;
    }
  };
  const HELD = 16;
  const shadowValue = (el) => {
    if (el.__vmuxHeld) return;
    const proto = el instanceof HTMLTextAreaElement ? HTMLTextAreaElement : HTMLInputElement;
    const own = Object.getOwnPropertyDescriptor(proto.prototype, 'value');
    if (!own || !own.set || !own.get) return;
    const held = [own.get.call(el)];
    el.__vmuxHeld = held;
    const remember = (text) => {
      if (held[held.length - 1] === text) return;
      held.push(text);
      if (held.length > HELD) held.shift();
    };
    el.addEventListener('input', () => remember(own.get.call(el)));
    Object.defineProperty(el, 'value', {
      configurable: true,
      get() {
        return own.get.call(this);
      },
      set(text) {
        const live = own.get.call(this);
        if (text !== live && held.includes(text)) return;
        own.set.call(this, text);
        remember(text);
      },
    });
  };
  const unshadowValue = (el) => {
    if (!el || !el.__vmuxHeld) return;
    delete el.value;
    delete el.__vmuxHeld;
  };
  document.addEventListener('focusin', (event) => {
    const el = event.target;
    if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) shadowValue(el);
  }, true);
  document.addEventListener('focusout', (event) => unshadowValue(event.target), true);
  document.addEventListener('paste', (event) => {
    const el = event.target;
    if (!(el instanceof HTMLInputElement) && !(el instanceof HTMLTextAreaElement)) return;
    const data = event.clipboardData;
    if (!data) return;
    const items = Array.from(data.items || []);
    const carriesImage = items.some((item) => item.type.startsWith('image/'))
      || (data.files && data.files.length > 0);
    if (carriesImage) event.preventDefault();
  }, true);
  const pumpEdits = async () => {
    let applied = false;
    for (;;) {
      try {
        const response = await fetch('/__edits', {
          headers: { 'x-vmux-applied': applied ? '1' : '0' },
        });
        applied = false;
        if (!response.ok) { await new Promise((r) => setTimeout(r, 50)); continue; }
        const frame = await response.arrayBuffer();
        if (frame.byteLength < 4) continue;
        const length = new DataView(frame).getUint32(0, true);
        const edits = frame.slice(4 + length);
        if (edits.byteLength) window.interpreter.run_from_bytes(edits);
        if (length) {
          const json = new TextDecoder().decode(new Uint8Array(frame, 4, length));
          for (const queued of JSON.parse(json)) applyDomRequest(queued);
        }
        applied = edits.byteLength > 0;
      } catch (e) {
        await new Promise((r) => setTimeout(r, 50));
      }
    }
  };
  const sameDocumentFragment = (href) => {
    const upToFragment = (url) => url.split('#')[0];
    try {
      const resolved = new URL(href, document.baseURI).href;
      return resolved.includes('#') && upToFragment(resolved) === upToFragment(location.href);
    } catch (e) {
      return false;
    }
  };
  const holdLinks = () => {
    window.interpreter.intercept_link_redirects = false;
    document.addEventListener('click', (event) => {
      if (event.defaultPrevented) return;
      const anchor = event.target instanceof Element ? event.target.closest('a') : null;
      if (!anchor) return;
      const href = anchor.getAttribute('href');
      if (href === null) return;
      if (sameDocumentFragment(href)) return;
      event.preventDefault();
      window.ipc.postMessage('link:' + href);
    });
  };
  window.vmuxWry = {
    start() { holdLinks(); pumpEdits(); },
    binEmit(buffer) { window.ipc.postMessage(toBase64(buffer)); },
    binListen(id, callback) {
      const existing = listeners.get(id) || [];
      existing.push(callback);
      listeners.set(id, existing);
    },
    _dispatch(id, base64) {
      const buffer = fromBase64(base64);
      for (const callback of listeners.get(id) || []) callback(buffer);
    },
  };
})();
"#;
