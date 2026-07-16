# TODO

Εκκρεμότητες για το cross-platform port (Windows/macOS). Το working tree έχει
3 commits (13–15 Ιουλ): cross-platform logs, icon drag-and-drop, authoritative
macOS stop.

## 1. Mac git index desync (Syncthing artifact)
- Το Syncthing συγχρονίζει live το `.git`, οπότε το `.git/index` του Mac έμεινε
  σε παλιό layout. Τα φυσικά αρχεία είναι σωστά (worktree == HEAD `8a08200`),
  χαμένη δουλειά δεν υπάρχει.
- Fix: `git reset` (mixed — δεν αγγίζει αρχεία) στο Mac για να καθαρίσει το status.
- Μελλοντικά: να βγει το `.git` από το Syncthing και sync μέσω `git push/pull`,
  αλλιώς θα ξανασυμβαίνει.

## 2. Testing σε πραγματικό hardware
- [x] **Logs (Bug 1)** — επιβεβαιωμένα δουλεύουν στο Mac (το `keydeck.log`
      γεμίζει με σωστά timestamps).
- [ ] **Authoritative stop (Bug 2)** — το binary το περιέχει, αλλά αδοκίμαστο το
      σενάριο "Stop → dot σβήνει, χωρίς respawn από KeepAlive".
- [ ] **Icon drag-and-drop** — αδοκίμαστο σε Mac & Windows.
- [ ] **Windows** — τίποτα δοκιμασμένο ακόμα (build + όλα τα features).

## 3. macOS keyboard injection δεν δουλεύει (TCC/Accessibility)
- Στο log: `ERROR: Failed to initialize input backend: the application does not
  have the permission to simulate input`.
- Χρειάζεται Accessibility grant + κοινό Team ID signing (Developer ID) μεταξύ
  daemon & app· ad-hoc signing δεν αρκεί.

## 4. Deferred / optional
- [ ] `~/keydeck-xbuild` (3.4G stale workspace) — διαγραφή αφού επιβεβαιωθεί.
- [ ] DMG finalize μέσω GitHub Action (AppleScript/Finder δεν παίζει over SSH).
