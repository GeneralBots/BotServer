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
    const msg = MessageFactory.text("");
    msg.value = value;
    msg.type = "event";
    msg.name = name;
    return dc.context.sendActivity(msg);
  }

  async routeNLP(dc: any, min: GBMinInstance, text: string): Promise<boolean> {

    // Invokes LUIS.

    const model = new LuisRecognizer({
      appId: min.instance.nlpAppId,
      subscriptionKey: min.instance.nlpSubscriptionKey,
      serviceEndpoint: min.instance.nlpServerUrl
    });
    let res = await model.recognize(dc.context);

    // Resolves intents returned from LUIS.

    let topIntent = LuisRecognizer.topIntent(res);
    if (topIntent) {
      var intent = topIntent;
      var entity =
        res.entities && res.entities.length > 0
          ? res.entities[0].entity.toUpperCase()
          : null;

      logger.info("NLP called:" + intent + ", " + entity);

      try {
        await dc.replace("/" + intent, res.entities);
      } catch (error) {
        let msg = `Error running NLP (${intent}): ${error}`;
        logger.info(msg);
        return Promise.reject(msg);
      }
      return Promise.resolve(true);
    } else {
      return Promise.resolve(false);
    }
  }
}
