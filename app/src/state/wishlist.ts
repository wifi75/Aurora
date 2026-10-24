import { create } from 'zustand';
import { EMPTY_WISHLIST, type Wishlist } from '../lib/wishlist';

/** La lista dei desiderati in uso, per evidenziare le stazioni in ogni elenco senza richiederla a ogni riga. */
interface WishlistState {
  list: Wishlist;
  set(list: Wishlist): void;
}

export const useWishlist = create<WishlistState>((set) => ({
  list: EMPTY_WISHLIST,
  set(list) {
    set({ list });
  },
}));
