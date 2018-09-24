import { IGBInstance } from "botlib";
/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' _ `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| ( ) |( (_) )  |
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

"use strict";

const logger = require("../../../src/logger");

import { GBCoreService } from "./GBCoreService";
import { IGBConversationalService } from "botlib";
import { GBMinInstance } from "botlib";
import { LuisRecognizer } from "botbuilder-ai";
import { MessageFactory } from "botbuilder";
import { Messages } from "../strings";
import { AzureText } from "pragmatismo-io-framework";
import { any } from "bluebird";
const Nexmo = require("nexmo");

export interface LanguagePickerSettings {
  defaultLocale?: string;
  supportedLocales?: string[];
}

export class GBConversationalService implements IGBConversationalService {
  coreService: GBCoreService;

  constructor(coreService: GBCoreService) {
    this.coreService = coreService;
  }

  getCurrentLanguage(dc: any) {
    return dc.context.activity.locale;
  }

  async sendEvent(dc: any, name: string, value: any): Promise<any> {
    if (dc.context.activity.channelId === "webchat") {
      const msg = MessageFactory.text("");
      msg.value = value;
      msg.type = "event";
      msg.name = name;
      return dc.context.sendActivity(msg);
    }
  }

  async sendSms(
    min: GBMinInstance,
    mobile: string,
    text: string
  ): Promise<any> {
    return new Promise((resolve, reject) => {
      const nexmo = new Nexmo({
        apiKey: min.instance.smsKey,
        apiSecret: min.instance.smsSecret
      });
      nexmo.message.sendSms(
        min.instance.smsServiceNumber,
        mobile,
        text,
        (err, data) => {
          if (err) {
            reject(err);
          } else {
            resolve(data);
          }
        }
      );
    });
  }

  async routeNLP(dc: any, min: GBMinInstance, text: string): Promise<boolean> {
    // Invokes LUIS.

    const model = new LuisRecognizer({
      appId: min.instance.nlpAppId,
      subscriptionKey: min.instance.nlpSubscriptionKey,
      serviceEndpoint: min.instance.nlpServerUrl
    });

    let nlp: any;
    try {
      nlp = await model.recognize(dc.context);
    } catch (error) {
      let msg = `Error calling NLP server, check if you have a published model and assigned keys on the service. Error: ${
        error.statusCode ? error.statusCode : ""
      } ${error.message}`;
      return Promise.reject(new Error(msg));
    }

    // Resolves intents returned from LUIS.

    let topIntent = LuisRecognizer.topIntent(nlp);
    if (topIntent) {
      var intent = topIntent;
      var entity =
        nlp.entities && nlp.entities.length > 0
          ? nlp.entities[0].entity.toUpperCase()
          : null;

      if (intent === "None") {
        return Promise.resolve(false);
      }

      logger.info("NLP called:" + intent + ", " + entity);

      try {
        await dc.replace("/" + intent, nlp.entities);
        return Promise.resolve(true);
      } catch (error) {
        let msg = `Error finding dialog associated to NLP event: ${intent}: ${
          error.message
        }`;
        return Promise.reject(new Error(msg));
      }
    }
    return Promise.resolve(false);
  }

  async checkLanguage(dc, min, text) {
    let locale = await AzureText.getLocale(
      min.instance.textAnalyticsKey,
      min.instance.textAnalyticsServerUrl,
      text
    );
    if (locale != dc.context.activity.locale.split("-")[0]) {
      switch (locale) {
        case "pt":
          dc.context.activity.locale = "pt-BR";
          await dc.context.sendActivity(Messages[locale].changing_language);
          break;
        case "en":
          dc.context.activity.locale = "en-US";
          await dc.context.sendActivity(Messages[locale].changing_language);
          break;
        default:
          await dc.context.sendActivity(`Unknown language: ${locale}`);
          break;
      }
    }
  }
}
