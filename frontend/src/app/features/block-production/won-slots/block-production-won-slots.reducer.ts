import { createReducer, on } from '@ngrx/store';
import { BlockProductionWonSlotsState } from '@block-production/won-slots/block-production-won-slots.state';
import { isDesktop, isMobile, sort, SortDirection, TableSort } from '@openmina/shared';
import { BlockProductionWonSlotsActions } from '@block-production/won-slots/block-production-won-slots.actions';
import {
  BlockProductionWonSlotsSlot,
  BlockProductionWonSlotsStatus,
} from '@shared/types/block-production/won-slots/block-production-won-slots-slot.type';

const initialState: BlockProductionWonSlotsState = {
  epoch: undefined,
  slots: [],
  filteredSlots: [],
  activeSlot: undefined,
  activeSlotRoute: undefined,
  openSidePanel: !isMobile(),
  filters: {
    accepted: true,
    orphaned: true,
    upcoming: true,
    discarded: true,
  },
  sort: {
    sortBy: 'slotTime',
    sortDirection: SortDirection.ASC,
  },
  serverResponded: false,
};

export const blockProductionWonSlotsReducer = createReducer(
  initialState,
  on(BlockProductionWonSlotsActions.init, (state, { activeSlotRoute }) => ({
    ...state,
    activeSlotRoute,
    serverResponded: false,
  })),
  on(BlockProductionWonSlotsActions.getSlotsSuccess, (state, { slots, epoch, activeSlot }) => ({
    ...state,
    slots,
    epoch,
    filteredSlots: filterSlots(sortSlots(slots, state.sort), state.filters),
    activeSlot,
    openSidePanel: state.activeSlot ? state.openSidePanel : isDesktop(),
    serverResponded: true,
  })),
  on(BlockProductionWonSlotsActions.setActiveSlot, (state, { slot }) => ({
    ...state,
    activeSlot: slot,
    activeSlotRoute: slot.globalSlot.toString(),
    openSidePanel: true,
  })),
  on(BlockProductionWonSlotsActions.sort, (state, { sort }) => ({
    ...state,
    sort,
    filteredSlots: filterSlots(sortSlots(state.slots, sort), state.filters),
  })),
  on(BlockProductionWonSlotsActions.changeFilters, (state, { filters }) => ({
    ...state,
    filters,
    filteredSlots: filterSlots(sortSlots(state.slots, state.sort), filters),
  })),
  on(BlockProductionWonSlotsActions.toggleSidePanel, state => ({
    ...state,
    openSidePanel: !state.openSidePanel,
    activeSlot: state.openSidePanel ? undefined : state.activeSlot,
    activeSlotRoute: state.openSidePanel ? undefined : state.activeSlotRoute,
  })),
  on(BlockProductionWonSlotsActions.close, () => initialState),
);

function sortSlots(node: BlockProductionWonSlotsSlot[], tableSort: TableSort<BlockProductionWonSlotsSlot>): BlockProductionWonSlotsSlot[] {
  return sort<BlockProductionWonSlotsSlot>(node, tableSort, ['message']);
}

function filterSlots(slots: BlockProductionWonSlotsSlot[], filters: BlockProductionWonSlotsState['filters']): BlockProductionWonSlotsSlot[] {
  return slots.filter(slot => {
    if (
      (filters.accepted && slot.status === BlockProductionWonSlotsStatus.Canonical)
      || (filters.orphaned && slot.status === BlockProductionWonSlotsStatus.Orphaned)
      || (filters.discarded && slot.status === BlockProductionWonSlotsStatus.Discarded)
      || slot.active
      || slot.status === BlockProductionWonSlotsStatus.Committed
    ) {
      return true;
    }
    return filters.upcoming && !slot.status || slot.status === BlockProductionWonSlotsStatus.Scheduled;
  });
}
