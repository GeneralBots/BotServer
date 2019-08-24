/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' _ `\ /'_`\   |
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
 * @fileoverview Conversation handling and external service calls.
 */

'use strict';

import { MessageFactory, RecognizerResult } from 'botbuilder';
import { LuisRecognizer } from 'botbuilder-ai';
import { GBDialogStep, GBLog, GBMinInstance, IGBConversationalService, IGBCoreService } from 'botlib';
import { AzureText } from 'pragmatismo-io-framework';
import { Messages } from '../strings';
const Nexmo = require('nexmo');

export interface LanguagePickerSettings {
  defaultLocale?: string;
  supportedLocales?: string[];
}

/**
 * Provides basic services for handling messages and dispatching to back-end
 * services like NLP or Search.
 */
export class GBConversationalService implements IGBConversationalService {
  public coreService: IGBCoreService;

  constructor(coreService: IGBCoreService) {
    this.coreService = coreService;
  }

  public getCurrentLanguage(step: GBDialogStep) {
    return step.context.activity.locale;
  }

  public async sendFile(min: GBMinInstance, step: GBDialogStep, url: string): Promise<any> {
    let mobile = step.context.activity.from.id;
    min.whatsAppDirectLine.sendFile(mobile, url);

  }

  public async sendEvent(step: GBDialogStep, name: string, value: Object): Promise<any> {
    if (step.context.activity.channelId === 'webchat') {
      const msg = MessageFactory.text('');
      msg.value = value;
      msg.type = 'event';
      msg.name = name;

      return step.context.sendActivity(msg);
    }
  }

  // tslint:disable:no-unsafe-any due to Nexmo.
  public async sendSms(min: GBMinInstance, mobile: string, text: string): Promise<any> {
    return new Promise(
      (resolve: any, reject: any): any => {
        const nexmo = new Nexmo({
          apiKey: min.instance.smsKey,
          apiSecret: min.instance.smsSecret
        });
        // tslint:disable-next-line:no-unsafe-any
        nexmo.message.sendSms(min.instance.smsServiceNumber, mobile, text, (err, data) => {
          if (err) {
            reject(err);
          } else {
            resolve(data);
          }
        });
      }
    );
  }
  // tslint:enable:no-unsafe-any

  public async routeNLP(step: GBDialogStep, min: GBMinInstance, text: string): Promise<boolean> {

    if (min.instance.nlpAppId === null){
      return false;
    }

    const model = new LuisRecognizer({
      applicationId: min.instance.nlpAppId,
      endpointKey: min.instance.nlpKey,
      endpoint: min.instance.nlpEndpoint
    });

    let nlp: RecognizerResult;
    try {
      nlp = await model.recognize(step.context);
    } catch (error) {
      // tslint:disable:no-unsafe-any
      if (error.statusCode === 404) {
        GBLog.warn('NLP application still not publish and there are no other options for answering.');

        return Promise.resolve(false);
      } else {
        const msg = `Error calling NLP, check if you have a published model and assigned keys. Error: ${
          error.statusCode ? error.statusCode : ''
          } {error.message; }`;

        return Promise.reject(new Error(msg));
      }
      // tslint:enable:no-unsafe-any
    }

    let nlpActive = false;

    Object.keys(nlp.intents).forEach((name) => {
      const score = nlp.intents[name].score;
      if (score > min.instance.nlpScore){
        nlpActive = true;
      }
    });

    // Resolves intents returned from LUIS.

    const topIntent = LuisRecognizer.topIntent(nlp);
    if (topIntent !== undefined && nlpActive) {
              
      const intent = topIntent;
      // tslint:disable:no-unsafe-any
      const firstEntity = nlp.entities && nlp.entities.length > 0 ? nlp.entities[0].entity.toUpperCase() : undefined;
      // tslint:ensable:no-unsafe-any

      if (intent === 'None') {
        return Promise.resolve(false);
      }

      GBLog.info(`NLP called: ${intent} ${firstEntity}`);

      try {
        await step.replaceDialog(`/${intent}`, nlp.entities);

        return Promise.resolve(true);
      } catch (error) {
        const msg = `Error finding dialog associated to NLP event: ${intent}: ${error.message}`;

        return Promise.reject(new Error(msg));
      }
    }

    return Promise.resolve(false);
  }

  public async checkLanguage(step: GBDialogStep, min, text) {
    const locale = await AzureText.getLocale(min.instance.textAnalyticsKey, min.instance.textAnalyticsEndpoint, text);
    if (locale !== step.context.activity.locale.split('-')[0]) {
      switch (locale) {
        case 'pt':
          step.context.activity.locale = 'pt-BR';
          await step.context.sendActivity(Messages[locale].changing_language);
          break;
        case 'en':
          step.context.activity.locale = 'en-US';
          await step.context.sendActivity(Messages[locale].changing_language);
          break;
        default:
          await step.context.sendActivity(`; Unknown; language: $;{locale;}`);
          break;
      }
    }
  }
}
