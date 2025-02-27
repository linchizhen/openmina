import { Directive, OnInit } from '@angular/core';
import { AppSelectors } from '@app/app.state';
import { AppMenu } from '@shared/types/app/app-menu.type';
import { MinaState } from '@app/app.setup';
import { MinaTableWrapper } from '@shared/base-classes/mina-table-wrapper.class';

@Directive()
export abstract class MinaTableRustWrapper<T extends object> extends MinaTableWrapper<T, MinaState> implements OnInit {

  override async ngOnInit(): Promise<void> {
    await super.ngOnInit();

    this.select(AppSelectors.menu, (menu: AppMenu) => {
      this.checkViewport(menu.isMobile);
    });
  }
}
