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
import { GBServer } from '../../../src/app';
import { GBWhatsappPackage } from '../../whatsapp.gblib';
const urlJoin = require('url-join');
const PasswordGenerator = require("strict-password-generator").default;
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

  public getNewMobileCode() {
    const passwordGenerator = new PasswordGenerator();
    const options = {
      upperCaseAlpha: false,
      lowerCaseAlpha: false,
      number: true,
      specialCharacter: false,
      minimumLength: 4,
      maximumLength: 4
    };
    let code = passwordGenerator.generatePassword(options);
    return code;
  }

  public getCurrentLanguage(step: GBDialogStep) {
    return step.context.activity.locale;
  }

  public async sendFile(min: GBMinInstance, step: GBDialogStep, mobile: string, url: string, caption: string): Promise<any> {
    if (step !== null) {
      mobile = step.context.activity.from.id;
    }
    const filename = url.substring(url.lastIndexOf('/') + 1);
    await min.whatsAppDirectLine.sendFileToDevice(mobile, url, filename, caption);

  }

  public async sendAudio(min: GBMinInstance, step: GBDialogStep, url: string): Promise<any> {
    const mobile = step.context.activity.from.id;
    await min.whatsAppDirectLine.sendAudioToDevice(mobile, url);
  }

  public async sendEvent(step: GBDialogStep, name: string, value: Object): Promise<any> {
    if (step.context.activity.channelId === 'webchat') {
      const msg = MessageFactory.text('');
      msg.value = value;
      msg.type = 'event';
      msg.name = name;

      return await step.context.sendActivity(msg);
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

  public async sendToMobile(min: GBMinInstance, mobile: string, message: string) {
    min.whatsAppDirectLine.sendToDevice(mobile, message);
  }

  // tslint:enable:no-unsafe-any

  public async sendMarkdownToMobile(min: GBMinInstance, step: GBDialogStep, mobile: string, text: string) {

    let sleep = (ms) => {
      return new Promise(resolve => {
        setTimeout(resolve, ms)
      })
    }
    enum State {
      InText,
      InImage,
      InImageBegin,
      InImageCaption,
      InImageAddressBegin,
      InImageAddressBody,
      InEmbedBegin,
      InEmbedEnd,
      InEmbedAddressBegin,
      InEmbedAddressEnd
    };
    let state = State.InText;
    let currentImage = '';
    let currentText = '';
    let currentCaption = '';
    let currentEmbedUrl = '';

    //![General Bots](/instance/images/gb.png)
    for (var i = 0; i < text.length; i++) {
      const c = text.charAt(i);

      switch (state) {
        case State.InText:
          if (c === '!') {
            state = State.InImageBegin;
          }
          else if (c === '[') {
            state = State.InEmbedBegin;
          }
          else {
            currentText = currentText.concat(c);
          }
          break;
        case State.InEmbedBegin:
          if (c === '=') {
            if (currentText !== '') {
              if (mobile === null) {
                await step.context.sendActivity(currentText);
              }
              else {
                this.sendToMobile(min, mobile, currentText);
              }
              await sleep(3000);
            }
            currentText = '';
            state = State.InEmbedAddressBegin;
          }
          
          break;
        case State.InEmbedAddressBegin:
          if (c === ']') {
            state = State.InEmbedEnd;
            let url = urlJoin(GBServer.globals.publicAddress, currentEmbedUrl);
            await this.sendFile(min, step, mobile, url, null);
            await sleep(5000);
            currentEmbedUrl = '';
          }
          else{
            currentEmbedUrl = currentEmbedUrl.concat(c);
          }
          break;
        case State.InEmbedEnd:
          if (c === ']') {
            state = State.InText;
          }
          break;
        case State.InImageBegin:
          if (c === '[') {
            if (currentText !== '') {
              if (mobile === null) {
                await step.context.sendActivity(currentText);
              }
              else {
                this.sendToMobile(min, mobile, currentText);
              }
              await sleep(3000);
            }
            currentText = '';
            state = State.InImageCaption;
          }
          else {
            state = State.InText;
            currentText = currentText.concat('!').concat(c);
          }
          break;
        case State.InImageCaption:
          if (c === ']') {
            state = State.InImageAddressBegin;
          }
          else {
            currentCaption = currentCaption.concat(c);
          }
          break;
        case State.InImageAddressBegin:
          if (c === '(') {
            state = State.InImageAddressBody;
          }
          break;
        case State.InImageAddressBody:
          if (c === ')') {
            state = State.InText;
            let url = urlJoin(GBServer.globals.publicAddress, currentImage);
            await this.sendFile(min, step, mobile, url, currentCaption);
            currentCaption = '';
            await sleep(5000);
            currentImage = '';
          }
          else {
            currentImage = currentImage.concat(c);
          }
          break;
      }

    }
    if (currentText !== '') {
      if (mobile === null) {
        await step.context.sendActivity(currentText);
      }
      else {
        this.sendToMobile(min, mobile, currentText);
      }
    }
  }

  public async routeNLP(step: GBDialogStep, min: GBMinInstance, text: string): Promise<boolean> {

    if (min.instance.nlpAppId === null) {
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
      if (score > min.instance.nlpScore) {
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
