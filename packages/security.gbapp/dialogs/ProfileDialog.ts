/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.com.br. All rights reserved.             |
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
| "General Bots" is a registered trademark of pragmatismo.com.br.             |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

/**
 * @fileoverview Main dialog for kb.gbapp
 */

'use strict';

import { GBLog, GBMinInstance, IGBDialog } from 'botlib';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService.js';
import { Messages } from '../strings.js';
import libphonenumber from 'google-libphonenumber';

/**
 * Dialogs for handling Menu control.
 */
export class ProfileDialog extends IGBDialog {
  public static getNameDialog(min: GBMinInstance) {
    return {
      id: '/profile_name',
      waterfall: [
        async step => {
          if (step.context.activity.channelId !== 'msteams' && process.env.ENABLE_AUTH) {
            return await step.beginDialog('/auth');
          } else {
            return await step.next(step.options);
          }
        },
        async step => {
          step.activeDialog.state.options = step.options;
          const locale = step.context.activity.locale;
          await step.prompt('textPrompt', Messages[locale].whats_name);
        },
        async step => {
          const locale = step.context.activity.locale;

          const fullName = text => {
            return text.match(/^[a-zA-Z]+(([',. -][a-zA-Z ])?[a-zA-Z]*)*$/gi);
          };

          const value = fullName(step.result);

          if (value === null) {
            await step.context.sendActivity(Messages[locale].validation_enter_name);
            await step.replaceDialog('/profile_name', step.activeDialog.state.options);
          } else {
            step.activeDialog.state.options.name = value[0];

            return await step.replaceDialog('/profile_mobile', step.activeDialog.state.options);
          }
        }
      ]
    };
  }

  public static getMobileDialog(min: GBMinInstance) {
    return {
      id: '/profile_mobile',
      waterfall: [
        async step => {
          if (step.context.activity.channelId !== 'msteams' && process.env.ENABLE_AUTH) {
            return await step.beginDialog('/auth');
          } else {
            return await step.next(step.options);
          }
        },
        async step => {
          step.activeDialog.state.options = step.options;
          const locale = step.context.activity.locale;
          await step.prompt('textPrompt', Messages[locale].whats_mobile);
        },
        async step => {
          const locale = step.context.activity.locale;
          let phoneNumber = step.context.activity.text;
          let p = libphonenumber.PhoneNumberUtil.getInstance();
          try {
            phoneNumber = p.parse(phoneNumber);
          } catch (error) {
            await step.context.sendActivity(Messages[locale].validation_enter_valid_mobile);

            return await step.replaceDialog('/profile_mobile', step.activeDialog.state.options);
          }
          if (!p.isPossibleNumber(phoneNumber)) {
            await step.context.sendActivity(Messages[locale].validation_enter_valid_mobile);

            return await step.replaceDialog('/profile_mobile', step.activeDialog.state.options);
          } else {
            step.activeDialog.state.options.mobile = `${phoneNumber.values_['1']}${phoneNumber.values_['2']}`;
            step.activeDialog.state.options.mobileCode = GBAdminService.getMobileCode();

            return await step.replaceDialog('/profile_mobile_confirm', step.activeDialog.state.options);
          }
        }
      ]
    };
  }

  public static getMobileConfirmDialog(min: GBMinInstance) {
    return {
      id: '/profile_mobile_confirm',
      waterfall: [
        async step => {
          if (step.context.activity.channelId !== 'msteams' && process.env.ENABLE_AUTH) {
            return await step.beginDialog('/auth');
          } else {
            return await step.next(step.options);
          }
        },
        async step => {
          step.activeDialog.state.options = step.options;
          const locale = step.context.activity.locale;
          const from = step.activeDialog.state.options.mobile.replace ('+', '');
          if (min.whatsAppDirectLine) {
            await min.whatsAppDirectLine.sendToDevice(
              from,
              `${step.activeDialog.state.options.mobileCode} is your General Bots creation code.`
            );
          } else {
            GBLog.info(`WhatsApp not configured. Here is the code: ${step.activeDialog.state.options.mobileCode}.`);
          }

          await step.prompt('textPrompt', Messages[locale].confirm_mobile);
        },
        async step => {
          const locale = step.context.activity.locale;

          if (step.result !== step.activeDialog.state.options.mobileCode) {
            await step.context.sendActivity(Messages[locale].confirm_mobile_again);

            return await step.replaceDialog('/profile_mobile_confirm', step.activeDialog.state.options);
          } else {
            await step.replaceDialog('/profile_email', step.activeDialog.state.options);
          }
        }
      ]
    };
  }

  public static getEmailDialog(min: GBMinInstance) {
    return {
      id: '/profile_email',
      waterfall: [
        async step => {
          if (step.context.activity.channelId !== 'msteams' && process.env.ENABLE_AUTH) {
            return await step.beginDialog('/auth');
          } else {
            return await step.next(step.options);
          }
        },
        async step => {
          const locale = step.context.activity.locale;
          await step.prompt('textPrompt', Messages[locale].whats_email);
        },
        async step => {
          const locale = step.context.activity.locale;

          const extractEntity = text => {
            return text.match(/([a-zA-Z0-9._-]+@[a-zA-Z0-9._-]+\.[a-zA-Z0-9_-]+)/gi);
          };

          const value = extractEntity(step.context.activity.originalText);

          if (value === null) {
            await step.context.sendActivity(Messages[locale].validation_enter_valid_email);
            await step.replaceDialog('/profile_email', step.activeDialog.state.options);
          } else {
            step.activeDialog.state.options.email = value[0];
            await step.replaceDialog(`/${step.activeDialog.state.options.nextDialog}`, step.activeDialog.state.options);
          }
        }
      ]
    };
  }
}
