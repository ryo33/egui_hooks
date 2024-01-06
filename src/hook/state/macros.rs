macro_rules! state_derive {
    ($state:ident) => {
        impl<T> std::borrow::Borrow<T> for $state<T> {
            #[inline]
            fn borrow(&self) -> &T {
                AsRef::<T>::as_ref(self)
            }
        }

        impl<T> AsRef<T> for $state<T> {
            #[inline]
            fn as_ref(&self) -> &T {
                std::ops::Deref::deref(self)
            }
        }

        impl<T: std::fmt::Display> std::fmt::Display for $state<T> {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                AsRef::<T>::as_ref(self).fmt(f)
            }
        }

        impl<T: std::fmt::Debug> std::fmt::Debug for $state<T> {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                AsRef::<T>::as_ref(self).fmt(f)
            }
        }

        impl<T: PartialEq> PartialEq for $state<T> {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                AsRef::<T>::as_ref(self).eq(&AsRef::<T>::as_ref(other))
            }
        }

        impl<T: Eq> Eq for $state<T> {}
        impl<T: PartialOrd> PartialOrd for $state<T> {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                AsRef::<T>::as_ref(self).partial_cmp(&AsRef::<T>::as_ref(other))
            }
        }
        impl<T: Ord> Ord for $state<T> {
            #[inline]
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                AsRef::<T>::as_ref(self).cmp(&AsRef::<T>::as_ref(other))
            }
        }

        impl<T: std::hash::Hash> std::hash::Hash for $state<T> {
            #[inline]
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                AsRef::<T>::as_ref(self).hash(state);
            }
        }
    };
}

pub(crate) use state_derive;
