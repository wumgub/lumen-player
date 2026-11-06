//! The play queue and its rules, ported from PlayerContext.tsx: shuffle that
//! ⏮ can retrace (and ⏭ replays forward again), repeat off/all/one, ⏮
//! restarting the song after three seconds, and the queue edits that keep the
//! shuffle history pointing at the right songs.
//!
//! This is plain data with no mpv in it, so it's tested on its own.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Repeat {
    #[default]
    Off,
    All,
    One,
}

impl Repeat {
    pub fn cycle(self) -> Repeat {
        match self {
            Repeat::Off => Repeat::All,
            Repeat::All => Repeat::One,
            Repeat::One => Repeat::Off,
        }
    }
}

/// What the player should do after a transport action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Start playing the queue entry at this index.
    Play(usize),
    /// Restart the current song.
    Restart,
    /// Stop; the queue ran out.
    Stop,
    /// Nothing to do.
    None,
}

/// Queue positions played so far under shuffle, so ⏮ can walk back through
/// what actually played. `forward` holds what ⏮ stepped back over, which ⏭
/// replays before picking new songs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct History {
    back: Vec<usize>,
    forward: Vec<usize>,
}

const HISTORY_MAX: usize = 500;

#[derive(Debug, Clone)]
pub struct Queue<T> {
    items: Vec<T>,
    index: Option<usize>,
    pub shuffle: bool,
    pub repeat: Repeat,
    history: History,
    rng: u64,
}

impl<T> Default for Queue<T> {
    fn default() -> Self {
        Queue::new()
    }
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9e37_79b9_7f4a_7c15);
        Queue {
            items: Vec::new(),
            index: None,
            shuffle: false,
            repeat: Repeat::Off,
            history: History::default(),
            rng: seed | 1,
        }
    }

    /// For tests: a fixed random sequence.
    pub fn with_seed(seed: u64) -> Self {
        Queue { rng: seed | 1, ..Queue::new() }
    }

    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn index(&self) -> Option<usize> {
        self.index
    }

    pub fn current(&self) -> Option<&T> {
        self.index.and_then(|i| self.items.get(i))
    }

    fn random_except(&mut self, len: usize, except: usize) -> usize {
        if len <= 1 {
            return 0;
        }
        loop {
            // xorshift64*
            self.rng ^= self.rng >> 12;
            self.rng ^= self.rng << 25;
            self.rng ^= self.rng >> 27;
            let r = (self.rng.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 33) as usize % len;
            if r != except {
                return r;
            }
        }
    }

    fn remember(&mut self, i: usize) {
        self.history.back.push(i);
        if self.history.back.len() > HISTORY_MAX {
            self.history.back.remove(0);
        }
    }

    fn reset_history(&mut self) {
        self.history = History::default();
    }

    fn remap_history(&mut self, f: impl Fn(usize) -> Option<usize>) {
        self.history.back = self.history.back.iter().filter_map(|&i| f(i)).collect();
        self.history.forward = self.history.forward.iter().filter_map(|&i| f(i)).collect();
    }

    /// Replace the queue and start at `start`.
    pub fn play(&mut self, items: Vec<T>, start: usize) -> Step {
        self.reset_history();
        self.items = items;
        if start < self.items.len() {
            self.index = Some(start);
            Step::Play(start)
        } else {
            self.index = None;
            Step::Stop
        }
    }

    /// What ⏭ (or the end of a song, with `auto`) would play, without moving.
    /// Under shuffle this can't be known in advance, so it returns None there
    /// unless ⏮ has left songs to replay.
    pub fn peek_next(&self, auto: bool) -> Option<usize> {
        let i = self.index?;
        let len = self.items.len();
        if len == 0 {
            return None;
        }
        if auto && self.repeat == Repeat::One {
            return Some(i);
        }
        if self.shuffle {
            return self.history.forward.last().copied().filter(|&f| f < len);
        }
        if i + 1 < len {
            Some(i + 1)
        } else if self.repeat == Repeat::All {
            Some(0)
        } else {
            None
        }
    }

    /// ⏭, or the end of a song when `auto`. Under shuffle, `planned` is the
    /// random pick made earlier (so gapless preloading can know it); without
    /// one, a fresh pick is made.
    pub fn next(&mut self, auto: bool, planned: Option<usize>) -> Step {
        let len = self.items.len();
        let Some(i) = self.index else { return Step::None };
        if len == 0 {
            return Step::None;
        }
        if self.repeat == Repeat::One && auto {
            return Step::Restart;
        }
        let ni = if self.shuffle {
            match self.history.forward.pop().filter(|&f| f < len) {
                Some(f) => f,
                None => match planned.filter(|&p| p < len) {
                    Some(p) => p,
                    None if len == 1 => i,
                    None => self.random_except(len, i),
                },
            }
        } else {
            self.history.forward.clear();
            if i + 1 < len {
                i + 1
            } else if self.repeat == Repeat::All {
                0
            } else {
                return Step::Stop;
            }
        };
        if ni != i {
            self.remember(i);
        }
        self.index = Some(ni);
        Step::Play(ni)
    }

    /// The random pick for the next song under shuffle, made ahead of time.
    pub fn plan_shuffle_next(&mut self) -> Option<usize> {
        let i = self.index?;
        let len = self.items.len();
        if !self.shuffle || len == 0 {
            return None;
        }
        if let Some(&f) = self.history.forward.last() {
            return (f < len).then_some(f);
        }
        Some(if len == 1 { i } else { self.random_except(len, i) })
    }

    /// ⏮. More than three seconds in, it restarts the song instead.
    pub fn prev(&mut self, position: f64) -> Step {
        let len = self.items.len();
        let Some(i) = self.index else { return Step::None };
        if len == 0 {
            return Step::None;
        }
        if position > 3.0 {
            return Step::Restart;
        }
        let ni = if self.shuffle {
            // Back through what shuffle actually played; with nothing earlier,
            // restart this one.
            match self.history.back.pop() {
                Some(b) if b < len => {
                    self.history.forward.push(i);
                    b
                }
                _ => return Step::Restart,
            }
        } else {
            i.saturating_sub(1)
        };
        self.index = Some(ni);
        Step::Play(ni)
    }

    /// Insert after the current song (`next`) or at the end. With nothing
    /// playing, the new songs become the queue and start.
    pub fn enqueue(&mut self, mut items: Vec<T>, next: bool) -> Step {
        if items.is_empty() {
            return Step::None;
        }
        let Some(i) = self.index.filter(|_| !self.items.is_empty()) else {
            return self.play(items, 0);
        };
        let at = if next { i + 1 } else { self.items.len() };
        let n = items.len();
        let tail = self.items.split_off(at);
        self.items.append(&mut items);
        self.items.extend(tail);
        self.remap_history(|j| Some(if j >= at { j + n } else { j }));
        Step::None
    }

    pub fn jump_to(&mut self, target: usize) -> Step {
        if target >= self.items.len() {
            return Step::None;
        }
        if let Some(cur) = self.index
            && target != cur
        {
            self.remember(cur);
            self.history.forward.clear();
        }
        self.index = Some(target);
        Step::Play(target)
    }

    /// Remove a queue entry. Removing the current song starts whatever slid
    /// into its place.
    pub fn remove(&mut self, target: usize) -> Step {
        if target >= self.items.len() {
            return Step::None;
        }
        self.items.remove(target);
        self.remap_history(|j| match j.cmp(&target) {
            std::cmp::Ordering::Equal => None,
            std::cmp::Ordering::Greater => Some(j - 1),
            std::cmp::Ordering::Less => Some(j),
        });
        let Some(cur) = self.index else { return Step::None };
        if target < cur {
            self.index = Some(cur - 1);
            Step::None
        } else if target == cur {
            if self.items.is_empty() {
                self.index = None;
                Step::Stop
            } else {
                let ni = cur.min(self.items.len() - 1);
                self.index = Some(ni);
                Step::Play(ni)
            }
        } else {
            Step::None
        }
    }

    /// Drop everything after the current song.
    pub fn clear_upcoming(&mut self) {
        if let Some(cur) = self.index {
            self.items.truncate(cur + 1);
            self.remap_history(|j| (j <= cur).then_some(j));
        }
    }

    pub fn set_shuffle(&mut self, on: bool) {
        // Each shuffle session retraces only its own path.
        self.reset_history();
        self.shuffle = on;
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.index = None;
        self.reset_history();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue(n: usize) -> Queue<usize> {
        let mut q = Queue::with_seed(42);
        q.play((0..n).collect(), 0);
        q
    }

    #[test]
    fn linear_next_and_prev() {
        let mut q = queue(3);
        assert_eq!(q.next(false, None), Step::Play(1));
        assert_eq!(q.next(true, None), Step::Play(2));
        assert_eq!(q.next(true, None), Step::Stop);
        assert_eq!(q.prev(1.0), Step::Play(1));
        assert_eq!(q.prev(5.0), Step::Restart);
        q.repeat = Repeat::All;
        q.jump_to(2);
        assert_eq!(q.peek_next(true), Some(0));
        assert_eq!(q.next(true, None), Step::Play(0));
        q.repeat = Repeat::One;
        assert_eq!(q.next(true, None), Step::Restart);
        assert_eq!(q.next(false, None), Step::Play(1));
    }

    #[test]
    fn shuffle_prev_retraces_and_next_replays() {
        let mut q = queue(20);
        q.set_shuffle(true);
        let mut played = vec![0];
        for _ in 0..5 {
            match q.next(false, None) {
                Step::Play(i) => played.push(i),
                s => panic!("{s:?}"),
            }
        }
        // ⏮ walks back through exactly what played.
        for want in played[..played.len() - 1].iter().rev().take(3) {
            assert_eq!(q.prev(0.5), Step::Play(*want));
        }
        // ⏭ replays what ⏮ stepped back over, in order.
        let n = played.len();
        for want in &played[n - 3..] {
            assert_eq!(q.next(false, None), Step::Play(*want));
        }
        // With nothing earlier, ⏮ restarts.
        let mut fresh = queue(5);
        fresh.set_shuffle(true);
        assert_eq!(fresh.prev(0.0), Step::Restart);
    }

    #[test]
    fn planned_shuffle_pick_is_used() {
        let mut q = queue(10);
        q.set_shuffle(true);
        let plan = q.plan_shuffle_next().unwrap();
        assert_ne!(plan, 0);
        assert_eq!(q.next(true, Some(plan)), Step::Play(plan));
        assert_eq!(q.prev(0.0), Step::Play(0));
    }

    #[test]
    fn queue_edits_keep_history_pointing_at_the_same_songs() {
        let mut q = Queue::with_seed(7);
        q.play(vec!['a', 'b', 'c', 'd'], 0);
        q.set_shuffle(true);
        q.jump_to(2); // played a, now c
        q.enqueue(vec!['x', 'y'], true); // a b c x y d
        assert_eq!(q.items(), &['a', 'b', 'c', 'x', 'y', 'd']);
        assert_eq!(q.prev(0.0), Step::Play(0)); // back to 'a'
        assert_eq!(q.current(), Some(&'a'));

        let mut q = Queue::with_seed(7);
        q.play(vec!['a', 'b', 'c', 'd'], 0);
        q.set_shuffle(true);
        q.jump_to(3); // remembered a
        q.remove(1); // a c d, current d at 2
        assert_eq!(q.index(), Some(2));
        assert_eq!(q.prev(0.0), Step::Play(0));
        assert_eq!(q.current(), Some(&'a'));
    }

    #[test]
    fn removing_the_current_song_plays_the_next_one() {
        let mut q = Queue::new();
        q.play(vec![1, 2, 3], 1);
        assert_eq!(q.remove(1), Step::Play(1));
        assert_eq!(q.current(), Some(&3));
        assert_eq!(q.remove(1), Step::Play(0));
        assert_eq!(q.remove(0), Step::Stop);
        assert_eq!(q.index(), None);
    }

    #[test]
    fn enqueue_with_nothing_playing_starts_it() {
        let mut q: Queue<i32> = Queue::new();
        assert_eq!(q.enqueue(vec![5, 6], false), Step::Play(0));
        assert_eq!(q.enqueue(vec![7], false), Step::None);
        assert_eq!(q.items(), &[5, 6, 7]);
        q.clear_upcoming();
        assert_eq!(q.items(), &[5]);
    }
}
