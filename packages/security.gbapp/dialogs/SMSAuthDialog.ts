/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.cloud. All rights reserved.         |
| Licensed under the AGPL-3.0.                                                |
|                                                                             |
| According to our dual licensing model, this program can be used either      |
| under the terms of the GNU Affero General Public License, version 3,        |
| or under a proprietary license.                                             |
|                                                                             |
| The texts of the GNU Affero General Public License with an additional       |
| permission and of our proprietary license can be found at and               |
| in the LICENSE file you have received along with this program.              |
|                                                                             |
| This program is distributed in the hope that it will be useful,             |
| but WITHOUT ANY WARRANTY, without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the                |
| GNU Affero General Public License for more details.                         |
|                                                                             |
| "General Bots" is a registered trademark of pragmatismo.cloud.             |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

/**
 * @fileoverview Dialog for handling OAuth scenarios.
 */

'use strict';

import { TokenResponse } from 'botbuilder';
import { GBLog, GBMinInstance, IGBDialog } from 'botlib';
import { Messages } from '../strings.js';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService.js';
import { SecService } from '../services/SecService.js';
import { GBLogEx } from '../../core.gbapp/services/GBLogEx.js';
/**
 * Dialogs for handling Menu control.
 */
export class SMSAuthDialog extends IGBDialog {
  public static getSMSAuthDialog(min: GBMinInstance) {
    return {
      id: '/smsauth',
      waterfall: [
        async (step) => {
          const msg = 'Por favor, qual o seu celular? Ex: 55 21 99999-0000.';
          step.activeDialog.state.resetInfo = {};
          return await min.conversationalService.prompt(min, step, msg);

        },
        async (step) => {

          await step.context.sendActivity('Por favor, digite o código enviado para seu celular.');

          const mobile = step.result.replace(/\+|\s|\-/g, '');
          const locale = step.context.activity.locale;
          step.activeDialog.state.resetInfo.mobile = mobile;

          // Generates a new mobile code.

          let code = GBAdminService.getMobileCode();
          GBLogEx.info(min, `SMS Auth: Generated new code: ${code} is being sent.`);
          step.activeDialog.state.resetInfo.sentCode = code;
          step.activeDialog.state.resetInfo.mobile = mobile;

          // Sends a confirmation SMS.

          await min.conversationalService.sendSms(min,
             mobile, Messages[locale].please_use_code(code));

          return await min.conversationalService.prompt(min, step, Messages[locale].confirm_mobile);
        },
        async (step) => {
          const typed = step.result;
          const locale = step.context.activity.locale;

          // Checks if the typed code is equal to the one
          // sent to the registered mobile.

          if (typed == step.activeDialog.state.resetInfo.sentCode) {
            let sec = new SecService();
            const member = step.context.activity.from;

            GBLogEx.info(min, `SMS Auth: User Authenticated.`);
            await step.context.sendActivity(Messages[locale].authenticated);

            return await step.endDialog(step.activeDialog.state.resetInfo.mobile);
          }
          else {
            await step.context.sendActivity(Messages[locale].not_authorized);
            return await step.endDialog(false);
          }
        }
      ]
    };
  }
}
