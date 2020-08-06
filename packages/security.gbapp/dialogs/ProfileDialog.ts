/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' v `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| (˅) |( (_) )  |
|   | ,__/'(_)  `\__,_)`\__  |(_) (_) (_)`\__,_)`\__)(_)(____/(_) (_)`\___/'  |
|   | |                ( )_) |                                                |
|   (_)                 \___/'                                                |
|                                                                             |
| General Bots Copyright (c) Pragmatismo.io. All rights reserved.             |
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
| "General Bots" is a registered trademark of Pragmatismo.io.                 |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

/**
 * @fileoverview Main dialog for kb.gbapp
 */

'use strict';

import urlJoin = require('url-join');

import { BotAdapter, CardFactory, MessageFactory } from 'botbuilder';
import { WaterfallDialog } from 'botbuilder-dialogs';
import { GBMinInstance, IGBDialog, GBLog } from 'botlib';
import { Messages } from '../strings';
import { GBConversationalService } from '../../core.gbapp/services/GBConversationalService';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService';
const phoneUtil = require('google-libphonenumber').PhoneNumberUtil.getInstance();
const phone = require('phone');

/**
 * Dialogs for handling Menu control.
 */
export class ProfileDialog extends IGBDialog {

  static getNameDialog(min: GBMinInstance) {

    return {
      id: '/profile_name', waterfall: [
        async step => {
          step.activeDialog.state.options = step.options;
          const locale = step.context.activity.locale;
          await step.prompt("textPrompt", Messages[locale].whats_name);
        },
        async step => {
          const locale = step.context.activity.locale;

          const fullName = (text) => {
            return text.match(/^[a-zA-Z]+(([',. -][a-zA-Z ])?[a-zA-Z]*)*$/gi);
          }

          const value = fullName(step.result);

          if (value === null) {
            await step.context.sendActivity(Messages[locale].validation_enter_name);
            await step.replaceDialog('/profile_name', step.activeDialog.state.options);
          }
          else {
            step.activeDialog.state.options.name = value[0];

            return await step.replaceDialog('/profile_mobile', step.activeDialog.state.options);

          }
        }]
    }
  }


  static getMobileDialog(min: GBMinInstance) {

    return {
      id: '/profile_mobile', waterfall: [
        async step => {
          step.activeDialog.state.options = step.options;
          const locale = step.context.activity.locale;
          await step.prompt("textPrompt", Messages[locale].whats_mobile);
        },
        async step => {
          const locale = step.context.activity.locale;
          let phoneNumber;
          try {
            phoneNumber = phone(step.result, 'BRA')[0]; // TODO: Use accordingly to the person.
            phoneNumber = phoneUtil.parse(phoneNumber);
          } catch (error) {
            await step.context.sendActivity(Messages[locale].validation_enter_valid_mobile);

            return await step.replaceDialog('/profile_mobile', step.activeDialog.state.options);
          }
          if (!phoneUtil.isPossibleNumber(phoneNumber)) {
            await step.context.sendActivity(Messages[locale].validation_enter_valid_mobile);

            return await step.replaceDialog('/profile_mobile', step.activeDialog.state.options);
          }
          else {
            step.activeDialog.state.options.mobile = `${phoneNumber.values_['1']}${phoneNumber.values_['2']}`;
            step.activeDialog.state.options.mobileCode = GBAdminService.getMobileCode();

            return await step.replaceDialog('/profile_mobile_confirm', step.activeDialog.state.options);
          }
        }]
    }
  }

  static getMobileConfirmDialog(min: GBMinInstance) {

    return {
      id: '/profile_mobile_confirm', waterfall: [
        async step => {
          step.activeDialog.state.options = step.options;
          const locale = step.context.activity.locale;
          let from = step.activeDialog.state.options.mobile;
          if (min.whatsAppDirectLine) {

            await min.whatsAppDirectLine.sendToDevice(from, `${step.activeDialog.state.options.mobileCode} is your General Bots creation code.`);
          } else {
            GBLog.info(`WhatsApp not configured. Here is the code: ${step.activeDialog.state.options.mobileCode}.`);
          }

          await step.prompt("textPrompt", Messages[locale].confirm_mobile);
        },
        async step => {
          const locale = step.context.activity.locale;

          if (step.result !== step.activeDialog.state.options.mobileCode) {
            await step.context.sendActivity(Messages[locale].confirm_mobile_again);

            return await step.replaceDialog('/profile_mobile_confirm', step.activeDialog.state.options);
          }
          else {
            await step.replaceDialog('/profile_email', step.activeDialog.state.options);
          }
        }]
    }
  }


  static getEmailDialog(min: GBMinInstance) {
    return {
      id: '/profile_email', waterfall: [
        async step => {
          const locale = step.context.activity.locale;
          await step.prompt("textPrompt", Messages[locale].whats_email);
        },
        async step => {
          const locale = step.context.activity.locale;

          const extractEntity = (text) => {
            return text.match(/([a-zA-Z0-9._-]+@[a-zA-Z0-9._-]+\.[a-zA-Z0-9_-]+)/gi);
          }

          const value = extractEntity(step.result);

          if (value === null) {
            await step.context.sendActivity(Messages[locale].validation_enter_valid_email);
            await step.replaceDialog('/profile_email', step.activeDialog.state.options);
          }
          else {
            step.activeDialog.state.options.email = value[0];
            await step.replaceDialog(`/${step.activeDialog.state.options.nextDialog}`, step.activeDialog.state.options);
          }
        }]
    }
  }
}
