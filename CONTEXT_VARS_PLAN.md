# Σχέδιο: Context variables (αναγνώριση context μέσα σε terminal)

## 1. Το μοντέλο

Ο daemon **δεν** κάνει introspection μόνος του τι τρέχει μέσα στο terminal.
Αντ' αυτού:

1. Ένας **εξωτερικός** μηχανισμός (δικό μας, event-based — π.χ. kitty watcher σαν το
   υπάρχον `layout_switch.py`) ανιχνεύει το context και **push-άρει μια μεταβλητή**
   στον daemon: `keydeck --set context=claude`.
2. Ο daemon κρατά ένα **key→value store** από τέτοιες μεταβλητές.
3. Τα pages κάνουν match σε συνδυασμό **window (class/title) + μεταβλητών**, με
   first-match-wins όπως ήδη γίνεται σήμερα με το `window_name`.

Γιατί έτσι:
- Το detection είναι εντελώς custom → ζει έξω από τον daemon, εκεί που ανήκει.
- Event-based (push), όχι polling. Τα υπάρχοντα `service`/`tick` είναι timer-based·
  αυτό είναι διαφορετικό μονοπάτι.
- Πιάνει και αλλαγές **μέσα στο ίδιο παράθυρο** (tab-switch, τρέξιμο `claude` in-place),
  που το window-focus event ΔΕΝ πιάνει — αρκεί το external hook να στέλνει το update.

Επιβεβαιώθηκε ζωντανά ότι το detection είναι εφικτό:
- kitty remote control (ήδη ενεργό): `kitty @ ls` → `is_focused`, `cwd`,
  `foreground_processes` ανά window. Ακριβές, ξεχωρίζει το ενεργό tab.
- `/proc` fallback: `stat` field 8 (tpgid) → foreground process, `/proc/<pid>/cwd` →
  git check. Terminal-agnostic αλλά δεν ξεχωρίζει ενεργό tab σε multiplexed terminals.

---

## 2. Κανάλι εντολών (νέο, event-based)

Σήμερα ο daemon δέχεται runtime input ΜΟΝΟ από: POSIX signals (SIGHUP=reload), USB,
focus listeners, tick, sleep. **Κανένα socket / D-Bus service / FIFO.** Τα CLI flags
(`--list`, `--info`, `--validate`) τρέχουν standalone — δεν μιλούν στον daemon.

### 2.1 Unix socket listener (Linux)

Νέο thread `spawn_context_listener(&tx, &still_active)`, spawn δίπλα στους άλλους στο
`server.rs:132`. Ανοίγει `UnixListener` στο `$XDG_RUNTIME_DIR/keydeck.sock`
(fallback `/tmp/keydeck-<uid>.sock`). Line-oriented protocol:

```
setvar context claude
setvar git 1
clearvar context
```

Κάθε γραμμή → `send(&tx, DeviceEvent::SetContextVar { key, value })`.
Καμία νέα εξάρτηση (std `UnixListener`). Windows/macOS: named pipe / localhost TCP
αργότερα — το `event::send` injection είναι πανομοιότυπο.

### 2.2 CLI: `keydeck --set`

Λεπτό frontend ώστε το external script να μην ξέρει το protocol:

```
keydeck --set context=claude      # συνδέεται στο socket, γράφει "setvar context claude"
keydeck --set context=            # άδειο value → clearvar
```

Αν ο daemon δεν τρέχει (δεν υπάρχει socket) → σιωπηλή αποτυχία / exit 0
(το watcher δεν πρέπει να σπάει αν το keydeck δεν τρέχει).

---

## 3. Event + state store

### 3.1 Νέο DeviceEvent variant (`event.rs`)

```rust
/// Set/clear an external context variable
SetContextVar { key: String, value: Option<String> },  // None = clear
```

### 3.2 Shared store (`server.rs`, δίπλα στο services_state)

```rust
let context_vars: Arc<RwLock<IndexMap<String, String>>> = Arc::new(RwLock::new(IndexMap::new()));
```

Threaded στους `PagedDevice` (όπως το `services_state` σήμερα) ώστε να το διαβάζουν
και το page-matching και ο render-time provider.

### 3.3 Event loop arm (`server.rs`, δίπλα στο FocusChanges arm)

```rust
DeviceEvent::SetContextVar { key, value } => {
    {
        let mut vars = context_vars.write().unwrap();
        match value {
            Some(v) => { vars.insert(key, v); }
            None    => { vars.shift_remove(&key); }
        }
    }
    // re-evaluate pages, ίδιο fan-out με το FocusChanges
    for device in devices.values() {
        device.focus_changed(&current_class, &current_title, false);
    }
}
```

Σημείωση: το `focus_changed` θα μετονομαστεί/επεκταθεί σε `reevaluate_pages` ώστε
να λαμβάνει υπόψη ΚΑΙ το context store (βλ. §5). Το debounce στο
`last_auto_target_page` (paged_device.rs:869) το κάνει ήδη ασφαλές να ξανακληθεί.

---

## 4. Config schema — ενοποιημένο `when`

Το `window_name` **παύει** να είναι ξεχωριστό πεδίο. Όλα τα κριτήρια auto-switch
μπαίνουν σε ένα γενικό `when` block, όπου το window είναι απλώς ένα φίλτρο δίπλα
στις context-μεταβλητές. First-match-wins όπως σήμερα — specific σελίδες πριν τη
generic fallback. Η λογική «kitty focused; αν var=X→A, Y→B, αλλιώς C» βγαίνει με τη σειρά.

```yaml
pages:
  A:
    when: { window: kitty, context: claude }   # window ΚΑΙ context
  B:
    when: { window: kitty, context: mc }
  C:
    when: { window: kitty }                    # μόνο window
  git-only:
    when: { git: "1" }                         # μόνο μεταβλητή, ανεξάρτητα από window
```

### Reserved (built-in) keys vs context vars

Ο διαχωρισμός γίνεται με **reserved key names**:

| Key | Πηγή | Match |
|---|---|---|
| `window` | focus state | substring σε class **Ή** title (ό,τι έκανε το `window_name`) |
| `class`  | focus state | substring μόνο σε class |
| `title`  | focus state | substring μόνο σε title |
| *οτιδήποτε άλλο* | context var store (§3.2) | **exact** match στην push-αρισμένη τιμή |

### Λογική: map = AND, λίστα = OR (DNF)

Μία αρχή, δύο επίπεδα λίστας — καλύπτει κάθε συνδυασμό χωρίς `all`/`any` keywords:

```yaml
# AND — πολλά κλειδιά στο ίδιο map
when: { window: kitty, context: claude }

# OR στην ίδια μεταβλητή — λίστα τιμών
when: { window: [kitty, konsole], context: mc }     # (kitty ή konsole) ΚΑΙ mc

# OR μεταξύ διαφορετικών ομάδων — λίστα από maps (κάθε map = AND· ματσάρει αν ΟΠΟΙΑΔΗΠΟΤΕ περνά)
when:
  - { window: konsole, context: mc }
  - { window: kitty,   context: claude }
```

Δηλαδή: **λίστα από maps** = OR ομάδων· **map** = AND κλειδιών· **λίστα τιμών σε κλειδί**
= OR τιμών. Είναι disjunctive normal form — πλήρης εκφραστικότητα, μία έννοια να θυμάσαι.
Το OR-μεταξύ-κλειδιών ΔΕΝ χρειάζεται templates ούτε tree keywords.

### Struct (keydeck-types/src/pages.rs)

```rust
// Αφαιρείται το pub window_name: Option<String>  (βλ. §4.1 legacy)
#[serde(skip_serializing_if = "Option::is_none")]
pub when: Option<When>,

/// Custom Deserialize: δέχεται είτε ένα map είτε λίστα από maps,
/// κανονικοποιεί σε Vec<group>. Κάθε group = ένα map. Κάθε τιμή = μία ή λίστα.
pub struct When { pub groups: Vec<IndexMap<String, WhenValue>> }

#[serde(untagged)]
pub enum WhenValue { One(String), Many(Vec<String>) }
```

### 4.1 Legacy `window_name` — backwards compatibility (απόφαση: auto-migrate στο UI)

- **Schema/UI:** clean break — το κανονικό πεδίο είναι πλέον μόνο `when`. Το config UI
  μετατρέπει αυτόματα `window_name: X` → `when: { window: X }` όταν ανοίγει/σώζει config.
- **Daemon loader:** για να ΜΗ σπάσει ένα παλιό config *πριν* ο χρήστης ανοίξει το UI,
  ο loader εξακολουθεί να **δέχεται** το legacy `window_name` στο ανάγνωσμα και το κάνει
  merge σε `when.window` (χωρίς να το ξαναγράφει). Υλοποίηση: κρατάμε ένα προσωρινό
  `#[serde(default, skip_serializing)] window_name: Option<String>` πεδίο και σε ένα
  post-deserialize βήμα (ή custom `Deserialize`) το χύνουμε μέσα στο `when`, μετά το
  αγνοούμε. Έτσι: daemon δουλεύει με παλιά+νέα configs, UI γράφει μόνο το νέο σχήμα.

---

## 5. Matching logic (`paged_device.rs:854-866`)

Σήμερα ελέγχει μόνο `page.window_name`. Νέα λογική: για κάθε σελίδα με `when`,
τρία φωλιασμένα iterators (DNF) — OR ομάδων → AND κλειδιών → OR τιμών. Built-in
keys διαβάζουν το focus state, όλα τα άλλα το context store:

```rust
for (name, page) in &self.pages.pages {
    let Some(when) = &page.when else { continue; };    // χωρίς when → όχι auto-switch
    let vars = self.context_vars.read().unwrap();
    let ok = when.groups.iter().any(|group|            // OR μεταξύ ομάδων
        group.iter().all(|(k, val)|                    // AND μεταξύ κλειδιών
            val.values().iter().any(|v| {              // OR μεταξύ τιμών του κλειδιού
                let v = v.to_lowercase();
                match k.as_str() {
                    "window" => class_lower.contains(&v) || title_lower.contains(&v),
                    "class"  => class_lower.contains(&v),
                    "title"  => title_lower.contains(&v),
                    _        => vars.get(k).map_or(false, |cur| cur.to_lowercase() == v),
                }
            })));
    if ok { target_page = Some(name.clone()); break; }
}
```

Ο `context_vars` (Arc) περνά στο `PagedDevice` όπως το `services_state`.
Το ίδιο loop τρέχει είτε από `FocusChanges` είτε από `SetContextVar`.

---

## 6. Render-time provider (bonus, σχεδόν δωρεάν)

`dynamic_params.rs:42` — νέο branch `var:` που διαβάζει το store:

```rust
"var" => context_vars.read().unwrap().get(arg).cloned()
                     .unwrap_or_else(|| ERROR_INDICATOR.to_string()),
```

Χρήση σε κείμενο κουμπιού: `${var:context}` → `claude`.
(Απαιτεί να περάσει το store στο `evaluate_dynamic_params`, όπως το services_state.)

---

## 7. Το external κομμάτι (δικό μας, εκτός daemon)

kitty watcher `context_watcher.py` (σαν το `layout_switch.py`), hook `on_focus_change`:

```python
def on_focus_change(boss, window, data):
    if not data.get('focused'):
        return
    fg = (window.child.foreground_processes or [{}])[-1]
    prog = (fg.get('cmdline') or ['?'])[0].rsplit('/', 1)[-1]
    subprocess.Popen(['keydeck', '--set', f'context={prog}'])
    # προαιρετικά: git check στο window.cwd → keydeck --set git=1/0
```

Το ίδιο hook που ήδη κάνει keyboard-layout switching μπορεί να κάνει και αυτό —
μηδέν επιπλέον polling.

---

## 8. UI (keydeck-config)

**Καθοριστικό:** το frontend είναι **εντελώς untyped** ως προς το config. Δεν υπάρχουν
TS interfaces για `Page`/`Pages` (καμία ts-rs/specta) — το config περνά το JS boundary
ως plain `any`. Όλο το YAML parse/serialize γίνεται **μόνο στο Rust**:
`load_config` → `serde_yaml_ng::from_str::<KeyDeckConf>` (src-tauri/src/lib.rs:114-136),
`save_config` → `to_string(&KeyDeckConf)` (lib.rs:150-196). Το JS απλώς μεταφέρει το object.

Συνέπειες:
- **Καμία αλλαγή TS types** — το `page.when` «απλώς δουλεύει» στο runtime.
- **Auto-migration δωρεάν στο Rust:** επειδή ΚΑΙ το save περνά από serde, η μετατροπή
  `window_name → when` (§4.1) γίνεται στο Rust layer· το JS δεν χρειάζεται migration
  κώδικα. Ένα παλιό config → πρώτο άνοιγμα/save μέσα από το UI → ξαναγράφεται ως `when`.
- **Validation:** σήμερα δεν υπάρχει `--validate` κλήση από το UI· η μόνη ανάδραση είναι
  error banner όταν το config αποτύχει να γίνει parse σε load/reload (+page.svelte:174-178,
  818-820). Ένα κακό `when` → serde error στο save/reload → banner. Προαιρετικά (φάση 2):
  νέο Tauri command γύρω από `keydeck --validate` για live feedback.

### Πού μπαίνει ο editor

`PageEditor.svelte` — αντικατάσταση του υπάρχοντος "Window Name" form-group
(γραμμές 147-198· setter `updateWindowName` 23-32, input 152-158, dropdown
`list_window_classes` 90-127). Το component έχει ήδη `page`, `config`, `groupKey`,
`pageName` και το `list_window_classes` Tauri command για autocomplete.

Δεύτερος consumer: `ButtonGrid.svelte:503-572` (`getPagesWithWindowNames` + fuzzy match)
— πρέπει να ενημερωθεί να διαβάζει το `when` (κλειδί `window`) αντί για `window_name`.

### Structured DNF editor (νέο `WhenEditor.svelte`)

Το DNF map-εται φυσικά σε repeaters — ίδια patterns που υπάρχουν ήδη στο PageEditor
για `on_tick` (add/update/remove, γραμμές 62-82) και `inherits` scalar-or-array (34-49):

```
When — auto-switch όταν ταιριάζει ΟΠΟΙΑΔΗΠΟΤΕ ομάδα:
┌─ Ομάδα 1  (όλα ΚΑΙ) ──────────────── [🗑] ┐
│  [window  ▾] [kitty          ] [+ τιμή]    │   ← λίστα τιμών = OR
│  [context ▾] [mc             ]             │
│  [+ συνθήκη]                                │
├────────────────────────────────────────────┤
│  Ομάδα 2  (όλα ΚΑΙ) ─────────────── [🗑]    │
│  [context ▾] [aider          ]             │
└────────────────────────────────────────────┘
[+ ομάδα (OR)]
```

- **Key control:** `<select>` με `window`/`class`/`title` + επιλογή «custom variable…»
  → free-text για context var name.
- **Value control:** input που δέχεται μία τιμή ή λίστα (chips ή normalize scalar-or-array
  όπως το `inherits`). Autocomplete μέσω `list_window_classes` όταν key ∈ {window,class,title}.
- Μέγεθος ~σαν `TemplateSelector`/`ActionEditor`, χωρίς backend/type churn πέρα από το
  Rust struct.

Η απλή περίπτωση (μία ομάδα, key `window`) πρέπει να παραμείνει one-liner στο UI ώστε να
μη γίνει βαρύτερη η συνηθισμένη χρήση — π.χ. αρχικά δείχνεται μόνο ένα row, το «+ ομάδα»
είναι για τις σπάνιες σύνθετες περιπτώσεις.

---

## 9. Αρχεία που αλλάζουν

| Αρχείο | Αλλαγή |
|---|---|
| `src/event.rs` | νέο variant `SetContextVar` |
| `src/listener_context.rs` (νέο) | Unix socket listener thread |
| `src/platform.rs` / `platform/*` | `spawn_context_listener` glue |
| `src/server.rs` | store, spawn, event-loop arm, thread το store στους devices |
| `src/main.rs` | `--set key=value` CLI (socket client) |
| `src/paged_device.rs` | `context_vars` field, extended matching, re-eval |
| `keydeck-types/src/pages.rs` | αφαίρεση `window_name` ως public πεδίο· νέο `when` (custom De/Serialize)· legacy `window_name` merge στο deserialize (§4.1) |
| `src/validate.rs` | validation του `when` (reject σε templates, όπως το παλιό `window_name`)· προειδοποίηση legacy |
| `src/dynamic_params.rs` | `var:` provider |
| `keydeck-config/src/lib/components/PageEditor.svelte` | αντικατάσταση Window Name group με `WhenEditor` |
| `keydeck-config/src/lib/components/WhenEditor.svelte` (νέο) | structured DNF editor |
| `keydeck-config/src/lib/components/ButtonGrid.svelte` | reader `window_name`→`when` (503-572) |
| external `context_watcher.py` | εκτός repo (dotfiles) |

---

## 10. Αποφάσεις (όλες κλεισμένες) — STATUS: ΥΛΟΠΟΙΗΘΗΚΕ

- [x] Ενοποιημένο `when` με reserved keys (`window`/`class`/`title`) + context vars.
      DNF: map=AND, λίστα=OR (λίστα τιμών + λίστα από maps). Χωρίς templates/tree.
- [x] `window_name`: clean break στο schema, auto-migrate (Rust round-trip: loader +
      UI `load_config`), legacy accept στο deserialize.
- [x] Socket path: `$XDG_RUNTIME_DIR/keydeck.sock` (fallback temp dir ανά χρήστη).
- [x] Οι μεταβλητές επιβιώνουν reload — το store φτιάχνεται μία φορά, ανεξάρτητο του config.
- [x] Windows/macOS: το socket δουλεύει σε unix (Linux+macOS)· Windows = no-op προς το παρόν.
- [x] Δεν υπάρχει persistence σε restart — εφήμερο, ο watcher το ξαναστέλνει στο επόμενο event.

### Επαλήθευση
- `cargo test -p keydeck-types` (6 tests): DNF De/Serialize, round-trip, migration ✓
- `cargo test --bin keydeck` (8 tests): `${var:}` provider ✓
- Live: socket → `SetContextVar` → store → page switch σε πραγματική συσκευή, με συνδυασμό
  window+context (`Mc` = kitty ∧ mc) ✓
- Frontend `npm run build` + tauri backend `cargo check` ✓

### Απομένει για τον χρήστη (deployment)
- Rebuild/deploy του daemon (`keydeck`) + config UI ώστε να αντικατασταθεί το τρέχον
  εγκατεστημένο binary — μέχρι τότε το `keydeck --set` αποτυγχάνει σιωπηλά.
- Ο kitty watcher (`~/.config/kitty/context_watcher.py`) έχει ήδη προστεθεί στο
  `kitty.conf` (`watcher context_watcher.py`)· ενεργοποιείται σε restart/reload του kitty.
