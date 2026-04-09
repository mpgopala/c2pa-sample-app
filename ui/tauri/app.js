// ── Tauri API helpers ─────────────────────────────────────────────────────────

const invoke = (...args) =>
    (window.__TAURI__?.core?.invoke ?? window.__TAURI_INTERNALS__?.invoke)(...args);

const dialogOpen = (opts) =>
    (window.__TAURI__?.dialog?.open ?? (() => Promise.resolve(null)))(opts);

const dialogSave = (opts) =>
    (window.__TAURI__?.dialog?.save ?? (() => Promise.resolve(null)))(opts);

// ── Constants ─────────────────────────────────────────────────────────────────

const ALGS = ['Es256','Es384','Es512','Ps256','Ps384','Ps512','Ed25519'];
const ALG_LABELS = {
    Es256: 'ES256 (ECDSA / SHA-256)', Es384: 'ES384 (ECDSA / SHA-384)',
    Es512: 'ES512 (ECDSA / SHA-512)', Ps256: 'PS256 (RSA-PSS / SHA-256)',
    Ps384: 'PS384 (RSA-PSS / SHA-384)', Ps512: 'PS512 (RSA-PSS / SHA-512)',
    Ed25519: 'Ed25519',
};
const PRESET_ASSERTIONS = [
    'c2pa.actions', 'c2pa.training-mining', 'stds.schema-org.CreativeWork',
    'c2pa.hash.data', 'c2pa.soft-binding',
];
const ACTION_TYPES = [
    'c2pa.created','c2pa.edited','c2pa.published','c2pa.converted',
    'c2pa.repackaged','c2pa.transcoded','c2pa.resized','c2pa.color_adjustments',
    'c2pa.cropped','c2pa.drawing','c2pa.filtered','c2pa.placed',
];
const DIGITAL_SOURCE_TYPES = [
    ['', '— none —'],
    ['http://cv.iptc.org/newscodes/digitalsourcetype/algorithmicMedia',   'Algorithmic Media (AI)'],
    ['http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia', 'Trained Algorithmic Media'],
    ['http://cv.iptc.org/newscodes/digitalsourcetype/digitalCapture',     'Digital Capture'],
    ['http://cv.iptc.org/newscodes/digitalsourcetype/digitalArt',         'Digital Art'],
    ['http://cv.iptc.org/newscodes/digitalsourcetype/compositeWithTrainedAlgorithmicMedia', 'Composite + AI'],
];
const RELATIONSHIPS = [
    ['componentOf', 'Component Of'], ['parentOf', 'Parent Of'], ['inputTo', 'Input To'],
];

// ── Global state ──────────────────────────────────────────────────────────────

const state = {
    page: 'sign',
    sign: {
        file: null, signedDest: '', manifestDest: '', title: '',
        assertions: [], ingredients: [],
        cert: '', key: '', alg: 'Es256',
        busy: false, result: { type: 'idle' },
        customLabel: '',
    },
    verify: {
        file: null, result: null,
        expanded: new Set(['root', 'root/active', 'valroot']),
        highlighted: null, recents: [],
    },
    settings: {
        trustLists: ['c2pa-trust-list.pem', 'custom-ca.pem'],
        configMode: 'file', configFile: 'config.toml', configJson: '',
        fetchRemote: true, timeout: 30,
    },
    log: {
        entries: [], autoScroll: true, filterText: '', filterLevel: null,
        height: 220, dragging: false, dragStartY: 0, dragStartH: 220, visible: true,
    },
};

// ── Utilities ─────────────────────────────────────────────────────────────────

function esc(str) {
    return String(str)
        .replace(/&/g,'&amp;').replace(/</g,'&lt;')
        .replace(/>/g,'&gt;').replace(/"/g,'&quot;');
}

function deriveSignedDest(src) {
    const dot = src.lastIndexOf('.');
    const slash = Math.max(src.lastIndexOf('/'), src.lastIndexOf('\\'));
    if (dot > slash) {
        return src.slice(0, dot) + '_signed' + src.slice(dot);
    }
    return src + '_signed';
}

function deriveManifestDest(src) {
    const dot = src.lastIndexOf('.');
    const slash = Math.max(src.lastIndexOf('/'), src.lastIndexOf('\\'));
    const stem = dot > slash ? src.slice(0, dot) : src;
    return stem + '.c2pa';
}

function basename(path) {
    return path.split(/[/\\]/).pop() || path;
}

function defaultDataFor(label) {
    switch (label) {
        case 'c2pa.actions':
            return { actions: [{ action: 'c2pa.created' }] };
        case 'c2pa.training-mining':
            return { use_train: false, use_mine: false };
        case 'stds.schema-org.CreativeWork':
            return { '@context': 'http://schema.org/', '@type': 'CreativeWork',
                     author: [{ '@type': 'Person', name: '' }], copyrightNotice: '' };
        default:
            return {};
    }
}

// ── Navigation ────────────────────────────────────────────────────────────────

function navigate(page) {
    state.page = page;
    document.querySelectorAll('.nav-tab').forEach(t => t.classList.remove('active'));
    document.getElementById('tab-' + page)?.classList.add('active');
    renderPage();
}

// ── Toast ─────────────────────────────────────────────────────────────────────

function showToast(type, title, message) {
    const container = document.getElementById('toast-container');
    const id = 'toast-' + Date.now();
    const div = document.createElement('div');
    div.id = id;
    div.className = 'toast toast-' + type;
    div.style.pointerEvents = 'all';
    div.innerHTML = `
        <div class="toast-body">
            <span class="toast-icon">${type === 'success' ? '✓' : '✕'}</span>
            <div>
                <div class="toast-title">${esc(title)}</div>
                <div class="toast-path">${esc(message)}</div>
            </div>
        </div>
        <button class="toast-dismiss">✕</button>`;
    div.querySelector('.toast-dismiss').onclick = () => div.remove();
    container.appendChild(div);
    setTimeout(() => div.remove(), 8000);
}

// ── Sign page ─────────────────────────────────────────────────────────────────

function renderAssertionEditor(a, idx) {
    switch (a.label) {
        case 'c2pa.actions': return renderActionsEditor(a, idx);
        case 'c2pa.training-mining': return renderTrainingEditor(a, idx);
        case 'stds.schema-org.CreativeWork': return renderCreativeWorkEditor(a, idx);
        default: return renderJsonEditor(a, idx);
    }
}

function renderActionsEditor(a, idx) {
    const actions = Array.isArray(a.data.actions) ? a.data.actions : [];
    const rows = actions.map((act, ai) => {
        const curAction = act.action || 'c2pa.created';
        const curDst = act.digitalSourceType || '';
        const actionOpts = ACTION_TYPES.map(t =>
            `<option value="${esc(t)}" ${t === curAction ? 'selected' : ''}>${esc(t)}</option>`
        ).join('');
        const dstOpts = DIGITAL_SOURCE_TYPES.map(([v, l]) =>
            `<option value="${esc(v)}" ${v === curDst ? 'selected' : ''}>${esc(l)}</option>`
        ).join('');
        return `<div class="action-row">
            <select class="field-select" data-action-field="action" data-assertion="${idx}" data-ai="${ai}">${actionOpts}</select>
            <select class="field-select" data-action-field="dst" data-assertion="${idx}" data-ai="${ai}">${dstOpts}</select>
            <button class="btn btn-sm btn-danger" data-remove-action="${idx}" data-ai="${ai}" ${actions.length <= 1 ? 'disabled' : ''}>✕</button>
        </div>`;
    }).join('');
    return `<div class="assertion-editor">${rows}
        <div class="add-row"><button class="btn btn-sm" data-add-action="${idx}">+ Action</button></div>
    </div>`;
}

function renderTrainingEditor(a, idx) {
    return `<div class="assertion-editor">
        <label class="checkbox-row">
            <input type="checkbox" data-train="${idx}" ${a.data.use_train ? 'checked' : ''}> Allow AI Training
        </label>
        <label class="checkbox-row">
            <input type="checkbox" data-mine="${idx}" ${a.data.use_mine ? 'checked' : ''}> Allow Data Mining
        </label>
    </div>`;
}

function renderCreativeWorkEditor(a, idx) {
    const author = a.data.author?.[0]?.name || '';
    const copyright = a.data.copyrightNotice || '';
    return `<div class="assertion-editor">
        <div class="field">
            <label>Author</label>
            <input type="text" value="${esc(author)}" placeholder="Name" data-cw-author="${idx}">
        </div>
        <div class="field">
            <label>Copyright Notice</label>
            <input type="text" value="${esc(copyright)}" placeholder="© 2024 Author" data-cw-copyright="${idx}">
        </div>
    </div>`;
}

function renderJsonEditor(a, idx) {
    const text = JSON.stringify(a.data, null, 2);
    return `<div class="assertion-editor">
        <div class="field">
            <label>Data (JSON)</label>
            <textarea class="json-textarea" rows="4" data-json-assertion="${idx}">${esc(text)}</textarea>
        </div>
    </div>`;
}

function renderSignPage() {
    const s = state.sign;
    const assertionItems = s.assertions.map((a, idx) => `
        <div class="assertion-item">
            <div class="assertion-header">
                <span class="assertion-label">${esc(a.label)}</span>
                <button class="btn btn-sm btn-danger" data-remove-assertion="${idx}">✕</button>
            </div>
            ${renderAssertionEditor(a, idx)}
        </div>`).join('');

    const ingredientItems = s.ingredients.map((ing, idx) => {
        const name = basename(ing.path);
        const relOpts = RELATIONSHIPS.map(([v, l]) =>
            `<option value="${esc(v)}" ${v === ing.relationship ? 'selected' : ''}>${esc(l)}</option>`
        ).join('');
        return `<div class="ingredient-item">
            <div class="ingredient-header">
                <span class="ingredient-name">${esc(name)}</span>
                <select class="field-select ingredient-rel" data-ing-rel="${idx}">${relOpts}</select>
                <button class="btn btn-sm btn-danger" data-remove-ing="${idx}">✕</button>
            </div>
            <div class="ingredient-path">${esc(ing.path)}</div>
        </div>`;
    }).join('');

    const algOpts = ALGS.map(a =>
        `<option value="${a}" ${a === s.alg ? 'selected' : ''}>${esc(ALG_LABELS[a])}</option>`
    ).join('');

    const presetOpts = PRESET_ASSERTIONS.map(l =>
        `<option value="${esc(l)}">${esc(l)}</option>`
    ).join('');

    const canAddManifest = s.file && s.manifestDest;
    const canSign = s.file && s.signedDest && s.cert && s.key;

    document.getElementById('page-content').innerHTML = `
        <div class="page-title">Sign Asset</div>
        <div class="two-panel">
            <div class="panel-left">
                <div class="drop-zone">
                    <p>Drop file here or</p>
                    <button class="btn btn-sm" id="sign-browse-src">Browse</button>
                </div>
                ${s.file ? `<div class="file-selected">✓ ${esc(s.file)}</div>` : ''}

                <div class="card">
                    <div class="card-title">Signer</div>
                    <div class="field">
                        <label>Algorithm</label>
                        <select class="field-select" id="sign-alg">${algOpts}</select>
                    </div>
                    <div class="field">
                        <label>Certificate (.pem)</label>
                        <div class="inline-row">
                            <input type="text" value="${esc(s.cert)}" placeholder="cert.pem" id="sign-cert" style="flex:1">
                            <button class="btn btn-sm" id="sign-browse-cert">Browse</button>
                        </div>
                    </div>
                    <div class="field">
                        <label>Private Key (.pem)</label>
                        <div class="inline-row">
                            <input type="text" value="${esc(s.key)}" placeholder="key.pem" id="sign-key" style="flex:1">
                            <button class="btn btn-sm" id="sign-browse-key">Browse</button>
                        </div>
                    </div>
                </div>
            </div>

            <div class="panel-right">
                <div class="card">
                    <div class="card-title">Manifest</div>
                    <div class="field">
                        <label>Title</label>
                        <input type="text" value="${esc(s.title)}" placeholder="Leave blank to use filename" id="sign-title">
                    </div>
                    <div class="field">
                        <label>Manifest Archive (.c2pa)</label>
                        <div class="inline-row">
                            <input type="text" value="${esc(s.manifestDest)}" placeholder="Derived automatically" id="sign-manifest-dest" style="flex:1">
                            <button class="btn btn-sm" id="sign-browse-manifest-dest">Browse</button>
                        </div>
                    </div>
                    <div class="field">
                        <label>Signed Output File</label>
                        <div class="inline-row">
                            <input type="text" value="${esc(s.signedDest)}" placeholder="Derived automatically" id="sign-signed-dest" style="flex:1">
                            <button class="btn btn-sm" id="sign-browse-signed-dest">Browse</button>
                        </div>
                    </div>
                </div>

                <div class="card">
                    <div class="card-title">Assertions</div>
                    ${assertionItems}
                    ${s.assertions.length === 0 ? '<p class="empty-state">No assertions — a bare manifest will be signed</p>' : ''}
                    <div class="assertion-add-row">
                        <select class="field-select" id="sign-preset-select">
                            <option value="">— preset —</option>
                            ${presetOpts}
                        </select>
                        <input type="text" class="custom-assertion-input" id="sign-custom-label"
                               value="${esc(s.customLabel)}" placeholder="custom label…">
                        <button class="btn btn-sm" id="sign-add-assertion">Add</button>
                    </div>
                </div>

                <div class="card">
                    <div class="card-title">Ingredients</div>
                    ${ingredientItems}
                    ${s.ingredients.length === 0 ? '<p class="empty-state">No ingredients</p>' : ''}
                    <div class="add-row">
                        <button class="btn btn-sm" id="sign-add-ingredient">+ Add Ingredient</button>
                    </div>
                </div>

                <div class="action-buttons">
                    <button class="btn btn-full" id="sign-add-manifest-btn"
                            ${!canAddManifest || s.busy ? 'disabled' : ''}
                            title="Export an unsigned .c2pa manifest archive">
                        ${s.busy ? 'Working…' : 'Add Manifest'}
                    </button>
                    <button class="btn btn-primary btn-full" id="sign-sign-btn"
                            ${!canSign || s.busy ? 'disabled' : ''}
                            title="Sign the asset and embed the manifest">
                        ${s.busy ? 'Working…' : 'Sign Asset'}
                    </button>
                </div>
            </div>
        </div>`;

    bindSignEvents();
}

function bindSignEvents() {
    const s = state.sign;

    // Source file
    document.getElementById('sign-browse-src')?.addEventListener('click', async () => {
        const path = await dialogOpen({
            filters: [
                { name: 'Assets', extensions: ['jpg','jpeg','png','mp4','mov','pdf','tiff','webp'] },
                { name: 'All files', extensions: ['*'] },
            ],
        });
        if (path) {
            s.file = path;
            s.signedDest = deriveSignedDest(path);
            s.manifestDest = deriveManifestDest(path);
            renderPage();
        }
    });

    // Cert / key
    document.getElementById('sign-cert')?.addEventListener('input', e => { s.cert = e.target.value; });
    document.getElementById('sign-key')?.addEventListener('input', e => { s.key = e.target.value; });
    document.getElementById('sign-browse-cert')?.addEventListener('click', async () => {
        const p = await dialogOpen({ filters: [{ name: 'PEM', extensions: ['pem','crt'] }] });
        if (p) { s.cert = p; renderPage(); }
    });
    document.getElementById('sign-browse-key')?.addEventListener('click', async () => {
        const p = await dialogOpen({ filters: [{ name: 'PEM', extensions: ['pem','key'] }] });
        if (p) { s.key = p; renderPage(); }
    });

    // Algorithm
    document.getElementById('sign-alg')?.addEventListener('change', e => { s.alg = e.target.value; });

    // Manifest fields
    document.getElementById('sign-title')?.addEventListener('input', e => { s.title = e.target.value; });
    document.getElementById('sign-manifest-dest')?.addEventListener('input', e => { s.manifestDest = e.target.value; });
    document.getElementById('sign-signed-dest')?.addEventListener('input', e => { s.signedDest = e.target.value; });

    document.getElementById('sign-browse-manifest-dest')?.addEventListener('click', async () => {
        const p = await dialogSave({ filters: [{ name: 'C2PA Archive', extensions: ['c2pa'] }] });
        if (p) { s.manifestDest = p; renderPage(); }
    });
    document.getElementById('sign-browse-signed-dest')?.addEventListener('click', async () => {
        const ext = s.file ? s.file.split('.').pop() : 'bin';
        const p = await dialogSave({ filters: [{ name: 'Same type', extensions: [ext] }] });
        if (p) { s.signedDest = p; renderPage(); }
    });

    // Preset assertion
    document.getElementById('sign-preset-select')?.addEventListener('change', e => {
        const v = e.target.value;
        if (v) { addAssertion(v); e.target.value = ''; }
    });

    // Custom assertion input + add button
    document.getElementById('sign-custom-label')?.addEventListener('input', e => { s.customLabel = e.target.value; });
    document.getElementById('sign-custom-label')?.addEventListener('keydown', e => {
        if (e.key === 'Enter') { addAssertion(s.customLabel.trim()); s.customLabel = ''; renderPage(); }
    });
    document.getElementById('sign-add-assertion')?.addEventListener('click', () => {
        addAssertion(s.customLabel.trim()); s.customLabel = ''; renderPage();
    });

    // Add ingredient
    document.getElementById('sign-add-ingredient')?.addEventListener('click', async () => {
        const p = await dialogOpen({ filters: [{ name: 'All files', extensions: ['*'] }] });
        if (p) {
            s.ingredients.push({ path: p, relationship: 'componentOf', title: null });
            renderPage();
        }
    });

    // Action buttons
    document.getElementById('sign-add-manifest-btn')?.addEventListener('click', doAddManifest);
    document.getElementById('sign-sign-btn')?.addEventListener('click', doSignAsset);

    // Event delegation for assertion editors and ingredient rows
    const panel = document.querySelector('.panel-right');
    panel?.addEventListener('change', e => {
        const t = e.target;
        // Assertion action field
        const af = t.dataset.actionField;
        if (af) {
            const idx = parseInt(t.dataset.assertion);
            const ai = parseInt(t.dataset.ai);
            const actions = s.assertions[idx].data.actions;
            if (af === 'action') {
                actions[ai].action = t.value;
            } else if (af === 'dst') {
                if (t.value) actions[ai].digitalSourceType = t.value;
                else delete actions[ai].digitalSourceType;
            }
        }
        // Ingredient relationship
        if (t.dataset.ingRel !== undefined) {
            s.ingredients[parseInt(t.dataset.ingRel)].relationship = t.value;
        }
        // Training-mining checkboxes
        if (t.dataset.train !== undefined) {
            s.assertions[parseInt(t.dataset.train)].data.use_train = t.checked;
        }
        if (t.dataset.mine !== undefined) {
            s.assertions[parseInt(t.dataset.mine)].data.use_mine = t.checked;
        }
    });
    panel?.addEventListener('input', e => {
        const t = e.target;
        // CreativeWork author
        if (t.dataset.cwAuthor !== undefined) {
            const idx = parseInt(t.dataset.cwAuthor);
            if (!s.assertions[idx].data.author) s.assertions[idx].data.author = [{ '@type': 'Person', name: '' }];
            s.assertions[idx].data.author[0].name = t.value;
        }
        if (t.dataset.cwCopyright !== undefined) {
            s.assertions[parseInt(t.dataset.cwCopyright)].data.copyrightNotice = t.value;
        }
        // JSON textarea
        if (t.dataset.jsonAssertion !== undefined) {
            try {
                s.assertions[parseInt(t.dataset.jsonAssertion)].data = JSON.parse(t.value);
            } catch (_) {}
        }
    });
    panel?.addEventListener('click', e => {
        const t = e.target;
        // Remove assertion
        if (t.dataset.removeAssertion !== undefined) {
            s.assertions.splice(parseInt(t.dataset.removeAssertion), 1);
            renderPage();
        }
        // Remove action
        if (t.dataset.removeAction !== undefined) {
            const idx = parseInt(t.dataset.removeAction);
            const ai = parseInt(t.dataset.ai);
            if (s.assertions[idx].data.actions.length > 1) {
                s.assertions[idx].data.actions.splice(ai, 1);
                renderPage();
            }
        }
        // Add action
        if (t.dataset.addAction !== undefined) {
            s.assertions[parseInt(t.dataset.addAction)].data.actions.push({ action: 'c2pa.created' });
            renderPage();
        }
        // Remove ingredient
        if (t.dataset.removeIng !== undefined) {
            s.ingredients.splice(parseInt(t.dataset.removeIng), 1);
            renderPage();
        }
    });
}

function addAssertion(label) {
    if (!label) return;
    if (state.sign.assertions.some(a => a.label === label)) return;
    state.sign.assertions.push({ label, data: defaultDataFor(label) });
    renderPage();
}

async function doAddManifest() {
    const s = state.sign;
    s.busy = true; renderPage();
    try {
        const path = await invoke('add_manifest_cmd', {
            params: {
                source: s.file,
                title: s.title || null,
                format: null,
                assertions: s.assertions.map(a => [a.label, a.data]),
                ingredients: s.ingredients,
            },
            dest: s.manifestDest,
        });
        showToast('success', 'Done', 'Manifest archive written to ' + path);
    } catch (e) {
        showToast('error', 'Failed', String(e));
    } finally {
        s.busy = false; renderPage();
    }
}

async function doSignAsset() {
    const s = state.sign;
    s.busy = true; renderPage();
    try {
        const path = await invoke('sign_asset_cmd', {
            params: {
                manifest: {
                    source: s.file,
                    title: s.title || null,
                    format: null,
                    assertions: s.assertions.map(a => [a.label, a.data]),
                    ingredients: s.ingredients,
                },
                dest: s.signedDest,
                cert_path: s.cert,
                key_path: s.key,
                alg: s.alg,
            },
        });
        showToast('success', 'Done', 'Signed asset written to ' + path);
    } catch (e) {
        showToast('error', 'Failed', String(e));
    } finally {
        s.busy = false; renderPage();
    }
}

// ── Verify page ───────────────────────────────────────────────────────────────

// Build a flat list of tree rows from a VerifyResult.
function buildFullTree(result) {
    const rows = [];
    const sections = new Set();

    function pushLeaf(id, label, depth, value) {
        const ingMatch = value && typeof value === 'string'
            ? value.match(/^self#jumbf=c2pa\.assertions\/(c2pa\.ingredient[^\s]*)/)
            : null;
        rows.push({ id, label, depth, isSection: false, value: String(value ?? ''), ingredientLink: ingMatch ? ingMatch[1] : null });
    }

    function pushSection(id, label, depth) {
        sections.add(id);
        rows.push({ id, label, depth, isSection: true, value: null });
    }

    function buildJsonRows(v, id, label, depth) {
        if (v && typeof v === 'object' && !Array.isArray(v)) {
            pushSection(id, label + ' {…}', depth);
            for (const [k, child] of Object.entries(v)) buildJsonRows(child, `${id}/${k}`, k, depth + 1);
        } else if (Array.isArray(v)) {
            pushSection(id, `${label} [${v.length}]`, depth);
            v.forEach((child, i) => buildJsonRows(child, `${id}/${i}`, `[${i}]`, depth + 1));
        } else if (v === null) {
            pushLeaf(id, label, depth, 'null');
        } else {
            pushLeaf(id, label, depth, v);
        }
    }

    function buildManifest(m, prefix, depth) {
        if (m.title)           pushLeaf(`${prefix}/title`,          'title',           depth, m.title);
        if (m.format)          pushLeaf(`${prefix}/format`,         'format',          depth, m.format);
        if (m.claim_generator) pushLeaf(`${prefix}/claim_generator`,'claim_generator', depth, m.claim_generator);
        pushLeaf(`${prefix}/instance_id`, 'instance_id', depth, m.instance_id);

        if (m.issuer || m.signing_time) {
            const sig = `${prefix}/signature`;
            pushSection(sig, 'signature', depth);
            if (m.issuer)        pushLeaf(`${sig}/issuer`, 'issuer', depth + 1, m.issuer);
            if (m.signing_time)  pushLeaf(`${sig}/time`,   'time',   depth + 1, m.signing_time);
        }

        const asnId = `${prefix}/assertions`;
        pushSection(asnId, `assertions (${m.assertions.length})`, depth);
        for (const a of m.assertions) {
            const aId = `${asnId}/${a.label}`;
            if (a.data && typeof a.data === 'object' && !Array.isArray(a.data)) {
                pushSection(aId, a.label, depth + 1);
                for (const [k, v] of Object.entries(a.data)) buildJsonRows(v, `${aId}/${k}`, k, depth + 2);
            } else {
                buildJsonRows(a.data, aId, a.label, depth + 1);
            }
        }

        const ingId = `${prefix}/ingredients`;
        pushSection(ingId, `ingredients (${m.ingredients.length})`, depth);
        for (const ing of m.ingredients) {
            const title = ing.title || '(untitled)';
            const iId = `${ingId}/${ing.instance_id}`;
            pushSection(iId, `${title} [${ing.relationship}]`, depth + 1);
            if (ing.data && typeof ing.data === 'object') {
                for (const [k, v] of Object.entries(ing.data)) buildJsonRows(v, `${iId}/${k}`, k, depth + 2);
            }
        }
    }

    pushSection('root', 'ManifestStore', 0);
    if (result.manifest) {
        pushSection('root/active', `${result.manifest.label} (active)`, 1);
        buildManifest(result.manifest, 'root/active', 2);
    }
    result.all_manifests.forEach((m, i) => {
        pushSection(`root/other/${i}`, m.label, 1);
        buildManifest(m, `root/other/${i}`, 2);
    });

    return { rows, sections };
}

function isVisible(id, sections, expanded) {
    const parts = id.split('/');
    for (let len = 1; len < parts.length; len++) {
        const ancestor = parts.slice(0, len).join('/');
        if (sections.has(ancestor) && !expanded.has(ancestor)) return false;
    }
    return true;
}

function findIngredientRowId(result, label) {
    if (result.manifest) {
        for (const ing of result.manifest.ingredients) {
            if (ing.label === label) return `root/active/ingredients/${ing.instance_id}`;
        }
    }
    result.all_manifests.forEach((m, i) => {
        for (const ing of m.ingredients) {
            if (ing.label === label) return `root/other/${i}/ingredients/${ing.instance_id}`;
        }
    });
    return null;
}

function ancestorIds(rowId) {
    const parts = rowId.split('/');
    return Array.from({ length: parts.length - 1 }, (_, i) => parts.slice(0, i + 1).join('/'));
}

function renderTreeRows(rows, sections, expanded, highlighted) {
    return rows
        .filter(r => isVisible(r.id, sections, expanded))
        .map(r => {
            const indent = r.depth * 16;
            const hl = r.id === highlighted ? ' tree-highlighted' : '';
            if (r.isSection) {
                const open = expanded.has(r.id);
                return `<div class="tree-node tree-section${hl}" style="padding-left:${indent}px" data-tree-id="${esc(r.id)}">
                    <span class="tree-icon">${open ? '▾' : '▸'}</span>${esc(r.label)}
                </div>`;
            } else {
                const ingLink = r.ingredientLink
                    ? `<span class="tree-ing-link" data-ing-link="${esc(r.ingredientLink)}">↗ ingredient</span>`
                    : '';
                return `<div class="tree-leaf" style="padding-left:${indent}px" data-tree-leaf="${esc(r.id)}">
                    <span class="tree-key">${esc(r.label)}</span>
                    <span class="tree-sep">: </span>
                    <span class="tree-value">${esc(r.value)}</span>
                    ${ingLink}
                </div>`;
            }
        }).join('');
}

function renderVerifyPage() {
    const v = state.verify;

    let validationBadge = '';
    let thumbnailHtml = '';
    let manifestTree = '<p style="color:var(--text-muted);font-style:italic;">Select a file to inspect</p>';
    let valTree = '';

    if (v.result) {
        const r = v.result;
        // Badge
        let badgeClass = 'status-unsigned', stateLabel = 'NO MANIFEST';
        if (r.state === 'trusted' || r.state === 'valid') { badgeClass = 'status-verified'; stateLabel = r.state.toUpperCase(); }
        else if (r.state === 'invalid') { badgeClass = 'status-tampered'; stateLabel = 'INVALID'; }
        const issuer = r.manifest?.issuer ?? null;
        const sigTime = r.manifest?.signing_time ?? null;
        validationBadge = `
            <div class="card">
                <div class="card-title">Validation</div>
                <div class="status-badge ${badgeClass}">
                    <span class="status-dot"></span>${stateLabel}
                </div>
                ${issuer ? `<div class="meta-row"><span class="meta-label">Signed by</span><span>${esc(issuer)}</span></div>` : ''}
                ${sigTime ? `<div class="meta-row"><span class="meta-label">Timestamp</span><span>${esc(sigTime)}</span></div>` : ''}
            </div>`;

        // Thumbnail
        if (r.manifest?.thumbnail_data_uri) {
            thumbnailHtml = `<div class="card">
                <div class="card-title">Thumbnail</div>
                <img src="${esc(r.manifest.thumbnail_data_uri)}" style="max-width:100%;border-radius:4px;" alt="Thumbnail">
            </div>`;
        }

        // Manifest tree
        if (!r.manifest) {
            manifestTree = '<p style="color:var(--text-secondary);font-style:italic;">No C2PA manifest found in this file.</p>';
        } else {
            const { rows, sections } = buildFullTree(r);
            const treeHtml = renderTreeRows(rows, sections, v.expanded, v.highlighted);
            manifestTree = `<div class="tree" id="manifest-tree">${treeHtml}</div>
                <div class="add-row"><button class="btn" id="verify-export-btn">Export Report</button></div>`;
        }

        // Validation status tree
        if (r.validation_statuses?.length > 0) {
            const valRows = [], valSections = new Set();
            function buildJsonRowsVal(val, id, label, depth) {
                if (val && typeof val === 'object' && !Array.isArray(val)) {
                    valSections.add(id);
                    valRows.push({ id, label: label + ' {…}', depth, isSection: true, value: null });
                    for (const [k, c] of Object.entries(val)) buildJsonRowsVal(c, `${id}/${k}`, k, depth + 1);
                } else if (Array.isArray(val)) {
                    valSections.add(id);
                    valRows.push({ id, label: `${label} [${val.length}]`, depth, isSection: true, value: null });
                    val.forEach((c, i) => buildJsonRowsVal(c, `${id}/${i}`, `[${i}]`, depth + 1));
                } else {
                    valRows.push({ id, label, depth, isSection: false, value: String(val ?? ''), ingredientLink: null });
                }
            }
            buildJsonRowsVal(r.validation_statuses, 'valroot', `ValidationStatuses (${r.validation_statuses.length})`, 0);
            valTree = `<div class="card">
                <div class="card-title">Validation Results</div>
                <div class="tree" id="val-tree">${renderTreeRows(valRows, valSections, v.expanded, null)}</div>
            </div>`;
        }
    }

    const recentItems = v.recents.slice(0, 5).map(r =>
        `<div class="list-item" style="cursor:pointer" data-open-recent="${esc(r.path)}">
            <span style="font-size:12px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${esc(r.name)}</span>
            <span style="font-size:11px;color:var(--text-secondary);flex-shrink:0;margin-left:8px;">${new Date(r.timestamp * 1000).toLocaleDateString()}</span>
        </div>`
    ).join('');

    document.getElementById('page-content').innerHTML = `
        <div class="page-title">Verify Asset</div>
        <div class="two-panel">
            <div class="panel-left">
                <div class="drop-zone">
                    <p>Drop file here or</p>
                    <button class="btn btn-sm" id="verify-browse-btn">Browse</button>
                </div>
                ${v.file ? `<div class="file-selected">✓ ${esc(v.file)}</div>` : ''}
                ${recentItems ? `<div class="card"><div class="card-title">Recent Files</div>${recentItems}</div>` : ''}
                ${thumbnailHtml}
                ${validationBadge}
            </div>
            <div class="panel-right">
                <div class="card">
                    <div class="card-title">Manifest Store</div>
                    ${manifestTree}
                </div>
                ${valTree}
            </div>
        </div>`;

    bindVerifyEvents();
}

function bindVerifyEvents() {
    const v = state.verify;

    document.getElementById('verify-browse-btn')?.addEventListener('click', async () => {
        const path = await dialogOpen({ filters: [{ name: 'All files', extensions: ['*'] }] });
        if (path) await openForVerify(path);
    });

    // Recent file click
    document.querySelector('.panel-left')?.addEventListener('click', e => {
        const path = e.target.closest('[data-open-recent]')?.dataset.openRecent;
        if (path) openForVerify(path);
    });

    // Tree toggle
    document.querySelectorAll('[data-tree-id]').forEach(el => {
        el.addEventListener('click', () => {
            const id = el.dataset.treeId;
            v.highlighted = null;
            if (v.expanded.has(id)) v.expanded.delete(id);
            else v.expanded.add(id);
            renderPage();
        });
    });

    // Ingredient link click
    document.querySelectorAll('[data-ing-link]').forEach(el => {
        el.addEventListener('click', e => {
            e.stopPropagation();
            const label = el.dataset.ingLink;
            if (!v.result) return;
            const targetId = findIngredientRowId(v.result, label);
            if (targetId) {
                for (const anc of ancestorIds(targetId)) v.expanded.add(anc);
                v.expanded.add(targetId);
                v.highlighted = targetId;
                renderPage();
                document.querySelector(`[data-tree-leaf="${CSS.escape(targetId)}"]`)?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
            }
        });
    });
}

async function openForVerify(path) {
    const v = state.verify;
    v.file = path;
    v.result = null;
    v.expanded = new Set(['root', 'root/active', 'valroot']);
    v.highlighted = null;
    renderPage();
    try {
        v.result = await invoke('verify_asset', { path });
        const entries = await invoke('push_recent_cmd', { path });
        v.recents = entries;
    } catch (e) {
        showToast('error', 'Verify failed', String(e));
    }
    renderPage();
}

// ── Settings page ─────────────────────────────────────────────────────────────

function renderSettingsPage() {
    const s = state.settings;

    const trustItems = s.trustLists.map((t, i) => `
        <div class="trust-item">
            <span>${esc(t)}</span>
            <button class="btn btn-sm btn-danger" data-remove-trust="${i}">Remove</button>
        </div>`).join('');

    document.getElementById('page-content').innerHTML = `
        <div class="settings-page">
            <div style="font-size:18px;font-weight:600">Settings</div>

            <div class="card">
                <div class="card-title">Trust Lists</div>
                ${trustItems}
                <div class="add-row">
                    <button class="btn btn-sm" id="settings-add-trust">+ Add</button>
                </div>
            </div>

            <div class="card">
                <div class="card-title">Configuration</div>
                <div class="radio-group">
                    <label>
                        <input type="radio" name="config-mode" value="file" ${s.configMode === 'file' ? 'checked' : ''}>
                        Load from file
                    </label>
                    ${s.configMode === 'file' ? `<div class="inline-row" style="margin-left:20px">
                        <input type="text" value="${esc(s.configFile)}" id="settings-config-file" style="flex:1">
                        <button class="btn btn-sm" id="settings-browse-config">Browse</button>
                    </div>` : ''}
                    <label>
                        <input type="radio" name="config-mode" value="json" ${s.configMode === 'json' ? 'checked' : ''}>
                        Load from JSON
                    </label>
                    ${s.configMode === 'json' ? `<textarea
                        id="settings-config-json"
                        placeholder='{ "trust": { ... } }'
                        style="margin-left:20px;width:calc(100% - 20px);height:80px;padding:8px;border:1px solid var(--border);border-radius:6px;font-family:monospace;font-size:12px"
                    >${esc(s.configJson)}</textarea>` : ''}
                </div>
            </div>

            <div class="card">
                <div class="card-title">HTTP Resolution</div>
                <div class="checkbox-group">
                    <label>
                        <input type="checkbox" id="settings-fetch-remote" ${s.fetchRemote ? 'checked' : ''}>
                        Fetch remote manifests automatically
                    </label>
                </div>
                <div class="inline-row" style="margin-top:10px">
                    <span style="font-size:13px;color:var(--text-secondary)">Timeout</span>
                    <input type="number" id="settings-timeout" value="${s.timeout}" min="1" max="300"
                           style="width:64px;padding:6px 8px;border:1px solid var(--border);border-radius:6px">
                    <span style="font-size:13px;color:var(--text-secondary)">seconds</span>
                </div>
            </div>

            <div class="settings-actions">
                <button class="btn" id="settings-reset">Reset to Default</button>
                <button class="btn btn-primary" id="settings-save">Save</button>
            </div>
        </div>`;

    // Bind events
    document.querySelectorAll('[data-remove-trust]').forEach(btn => {
        btn.addEventListener('click', () => {
            s.trustLists.splice(parseInt(btn.dataset.removeTrust), 1);
            renderPage();
        });
    });
    document.getElementById('settings-add-trust')?.addEventListener('click', () => {
        s.trustLists.push('new-trust.pem');
        renderPage();
    });
    document.querySelectorAll('input[name="config-mode"]').forEach(r => {
        r.addEventListener('change', () => { s.configMode = r.value; renderPage(); });
    });
    document.getElementById('settings-config-file')?.addEventListener('input', e => { s.configFile = e.target.value; });
    document.getElementById('settings-browse-config')?.addEventListener('click', async () => {
        const p = await dialogOpen({ filters: [{ name: 'Config', extensions: ['toml','json','yaml'] }] });
        if (p) { s.configFile = p; renderPage(); }
    });
    document.getElementById('settings-config-json')?.addEventListener('input', e => { s.configJson = e.target.value; });
    document.getElementById('settings-fetch-remote')?.addEventListener('change', e => { s.fetchRemote = e.target.checked; });
    document.getElementById('settings-timeout')?.addEventListener('input', e => {
        const v = parseInt(e.target.value);
        if (!isNaN(v)) s.timeout = v;
    });
    document.getElementById('settings-reset')?.addEventListener('click', () => {
        Object.assign(state.settings, {
            trustLists: [], configMode: 'file', configFile: 'config.toml',
            configJson: '', fetchRemote: true, timeout: 30,
        });
        renderPage();
    });
    document.getElementById('settings-save')?.addEventListener('click', () => {
        showToast('success', 'Settings saved', 'Configuration updated.');
    });
}

// ── Page dispatch ─────────────────────────────────────────────────────────────

function renderPage() {
    switch (state.page) {
        case 'sign':     renderSignPage();     break;
        case 'verify':   renderVerifyPage();   break;
        case 'settings': renderSettingsPage(); break;
    }
}

// ── Log pane ──────────────────────────────────────────────────────────────────

function formatTs(tsMs) {
    const secs = Math.floor(tsMs / 1000);
    const ms = tsMs % 1000;
    const h = Math.floor(secs / 3600) % 24;
    const m = Math.floor(secs / 60) % 60;
    const s = secs % 60;
    return String(h).padStart(2,'0') + ':' + String(m).padStart(2,'0') + ':' +
           String(s).padStart(2,'0') + '.' + String(ms).padStart(3,'0');
}

const LEVEL_ORDER = { error: 0, warn: 1, info: 2, debug: 3, trace: 4 };

function levelCss(level) {
    switch (level) {
        case 'error': return 'log-error';
        case 'warn':  return 'log-warn';
        case 'info':  return 'log-info';
        case 'debug': return 'log-debug';
        default:      return 'log-trace';
    }
}

function renderLogPane() {
    const logEl = document.getElementById('log-pane');
    const l = state.log;
    logEl.style.height = l.height + 'px';

    logEl.innerHTML = `
        <div class="log-header">
            <span class="log-header-title">Log</span>
            <div class="log-header-actions">
                <label class="log-autoscroll-label">
                    <input type="checkbox" id="log-autoscroll" ${l.autoScroll ? 'checked' : ''}> Auto-scroll
                </label>
                <button class="btn btn-sm" id="log-clear">Clear</button>
            </div>
        </div>
        <div class="log-filter-row">
            <input type="text" class="log-filter-input" id="log-filter" placeholder="Filter logs…" value="${esc(l.filterText)}">
            <select class="log-filter-select" id="log-level-filter">
                <option value="" ${!l.filterLevel ? 'selected' : ''}>All levels</option>
                <option value="trace" ${l.filterLevel === 'trace' ? 'selected' : ''}>Trace+</option>
                <option value="debug" ${l.filterLevel === 'debug' ? 'selected' : ''}>Debug+</option>
                <option value="info"  ${l.filterLevel === 'info'  ? 'selected' : ''}>Info+</option>
                <option value="warn"  ${l.filterLevel === 'warn'  ? 'selected' : ''}>Warn+</option>
                <option value="error" ${l.filterLevel === 'error' ? 'selected' : ''}>Error only</option>
            </select>
        </div>
        <div class="log-entries" id="log-entries"></div>`;

    appendLogEntries(false);

    document.getElementById('log-autoscroll')?.addEventListener('change', e => { l.autoScroll = e.target.checked; });
    document.getElementById('log-clear')?.addEventListener('click', () => { l.entries = []; appendLogEntries(false); });
    document.getElementById('log-filter')?.addEventListener('input', e => { l.filterText = e.target.value; appendLogEntries(false); });
    document.getElementById('log-level-filter')?.addEventListener('change', e => {
        l.filterLevel = e.target.value || null;
        appendLogEntries(false);
    });
}

function appendLogEntries(scrollToBottom) {
    const l = state.log;
    const container = document.getElementById('log-entries');
    if (!container) return;

    const text = l.filterText.toLowerCase();
    const maxLevel = l.filterLevel ? LEVEL_ORDER[l.filterLevel] : 4;
    const visible = l.entries.filter(e => {
        if (LEVEL_ORDER[e.level] > maxLevel) return false;
        if (text) {
            const m = e.message.toLowerCase(), t = e.target.toLowerCase();
            if (!m.includes(text) && !t.includes(text)) return false;
        }
        return true;
    });

    if (visible.length === 0) {
        container.innerHTML = `<div class="log-empty">${l.entries.length === 0 ? 'No log entries yet.' : 'No entries match the current filter.'}</div>`;
        return;
    }

    container.innerHTML = visible.map(e => {
        const css = levelCss(e.level);
        return `<div class="log-row ${css}">
            <span class="log-ts">${formatTs(e.ts_ms)}</span>
            <span class="log-level ${css}-badge">${e.level.toUpperCase()}</span>
            <span class="log-target">${esc(e.target)}</span>
            <span class="log-msg">${esc(e.message)}</span>
        </div>`;
    }).join('');

    if (scrollToBottom || l.autoScroll) {
        container.scrollTop = container.scrollHeight;
    }
}

// ── Log resize ────────────────────────────────────────────────────────────────

function initLogResize() {
    const handle = document.getElementById('log-resize-handle');
    const l = state.log;

    handle.addEventListener('mousedown', e => {
        e.preventDefault();
        l.dragging = true;
        l.dragStartY = e.clientY;
        l.dragStartH = l.height;
    });

    document.addEventListener('mousemove', e => {
        if (!l.dragging) return;
        const dy = e.clientY - l.dragStartY;
        l.height = Math.max(80, Math.min(600, l.dragStartH - dy));
        document.getElementById('log-pane').style.height = l.height + 'px';
    });

    document.addEventListener('mouseup', () => { l.dragging = false; });
}

// ── Log polling ───────────────────────────────────────────────────────────────

async function pollLogs() {
    try {
        const newEntries = await invoke('drain_logs_cmd');
        if (newEntries.length > 0) {
            state.log.entries.push(...newEntries);
            if (state.log.entries.length > 500) {
                state.log.entries.splice(0, state.log.entries.length - 500);
            }
            appendLogEntries(false);
        }
    } catch (_) {}
}

// ── Init ──────────────────────────────────────────────────────────────────────

async function init() {
    // Nav tab clicks
    document.getElementById('tab-sign')?.addEventListener('click', () => navigate('sign'));
    document.getElementById('tab-verify')?.addEventListener('click', () => navigate('verify'));
    document.getElementById('tab-settings')?.addEventListener('click', () => navigate('settings'));

    // Set initial active tab
    document.getElementById('tab-sign')?.classList.add('active');

    // Load recents
    try {
        state.verify.recents = await invoke('load_recents_cmd');
    } catch (_) {}

    renderPage();
    renderLogPane();
    initLogResize();

    setInterval(pollLogs, 200);
}

init();
