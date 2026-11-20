using System;
using System.Runtime.CompilerServices;

namespace SlimeOptimizer
{
    internal sealed class FastObjCache<TKey, TVal>
        where TKey : class
        where TVal : class
    {
        private struct Entry
        {
            public int Hash;
            public TKey Key;
            public TVal Value;
        }

        private Entry[] _entries;
        private int _mask;
        private int _count;
        private int _resizeThreshold;
        private readonly bool _isUnityObject;

        public FastObjCache(int initialCapacity = 256)
        {
            int cap = 16;
            while (cap < initialCapacity)
            {
                cap <<= 1;
            }

            _entries = new Entry[cap];
            _mask = cap - 1;
            _resizeThreshold = (cap * 3) >> 2; // 75%
            _isUnityObject = typeof(UnityEngine.Object).IsAssignableFrom(typeof(TKey));
        }

        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public bool TryGetValue(TKey key, out TVal value)
        {
            if (ReferenceEquals(key, null))
            {
                value = null;
                return false;
            }

            int hash = RuntimeHelpers.GetHashCode(key) & 0x7FFFFFFF;
            Entry[] entries = _entries;
            int mask = _mask;
            int idx = hash & mask;

            while (true)
            {
                TKey k = entries[idx].Key;
                if (ReferenceEquals(k, null))
                {
                    value = null;
                    return false;
                }

                if (entries[idx].Hash == hash && ReferenceEquals(k, key))
                {
                    value = entries[idx].Value;
                    return true;
                }

                idx = (idx + 1) & mask;
            }
        }

        public TVal GetOrCreate(TKey key, Func<TKey, TVal> factory)
        {
            if (TryGetValue(key, out TVal existing))
            {
                return existing;
            }

            TVal created = factory(key);
            Set(key, created);
            return created;
        }

        public void Set(TKey key, TVal value)
        {
            if (ReferenceEquals(key, null))
            {
                return;
            }

            if (_count >= _resizeThreshold)
            {
                PruneOrGrow();
            }

            int hash = RuntimeHelpers.GetHashCode(key) & 0x7FFFFFFF;
            InsertRaw(_entries, _mask, hash, key, value, ref _count);
        }

        private static void InsertRaw(Entry[] entries, int mask, int hash, TKey key, TVal value, ref int count)
        {
            int idx = hash & mask;
            while (true)
            {
                TKey k = entries[idx].Key;
                if (ReferenceEquals(k, null))
                {
                    entries[idx].Hash = hash;
                    entries[idx].Key = key;
                    entries[idx].Value = value;
                    count++;
                    return;
                }

                if (entries[idx].Hash == hash && ReferenceEquals(k, key))
                {
                    entries[idx].Value = value;
                    return;
                }

                idx = (idx + 1) & mask;
            }
        }

        private void PruneOrGrow()
        {
            Entry[] old = _entries;
            int newCap = old.Length;

            if (_isUnityObject)
            {
                int alive = 0;
                for (int i = 0; i < old.Length; i++)
                {
                    TKey k = old[i].Key;
                    if (!ReferenceEquals(k, null) && (k as UnityEngine.Object) != null)
                    {
                        alive++;
                    }
                }

                if (alive * 2 < old.Length)
                {
                    // Rehash in-place to new same-size table without growing memory
                    Rehash(newCap);
                    return;
                }
            }

            Rehash(newCap << 1);
        }

        private void Rehash(int newCap)
        {
            Entry[] old = _entries;
            Entry[] next = new Entry[newCap];
            int mask = newCap - 1;
            int count = 0;

            for (int i = 0; i < old.Length; i++)
            {
                TKey k = old[i].Key;
                if (ReferenceEquals(k, null))
                {
                    continue;
                }

                if (_isUnityObject && (k as UnityEngine.Object) == null)
                {
                    continue;
                }

                InsertRaw(next, mask, old[i].Hash, k, old[i].Value, ref count);
            }

            _entries = next;
            _mask = mask;
            _count = count;
            _resizeThreshold = (newCap * 3) >> 2;
        }
    }
}
