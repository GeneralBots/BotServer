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

'use strict';

import { GBMinInstance } from 'botlib';
import OpenAI from "openai";
import { ChatGPTAPIBrowser, getOpenAIAuth } from 'chatgpt'
import { CollectionUtil } from 'pragmatismo-io-framework';

export class ChatServices {

  public static async sendMessage(min: GBMinInstance, text: string) {
    let key;
    if (process.env.OPENAI_KEY) {
      key = process.env.OPENAI_KEY;
    }
    else {
      key = min.core.getParam(min.instance, 'Open AI Key', null);
    }

    if (!key) {
      throw new Error('Open AI Key not configured in .gbot.');
    }
    let functions = [];
    
    await CollectionUtil.asyncForEach(Object.values(min.scriptMap), async script => {
  
      functions.push({
          "name": "get_current_weather",
          "description": "Get the current weather in a given location",
          "parameters": {
              "type": "object",
              "properties": {
                  "location": {
                      "type": "string",
                      "description": "The city and state, e.g. San Francisco, CA",
                  },
                  "unit": {"type": "string", "enum": ["celsius", "fahrenheit"]},
              },
              "required": ["location"],
          } });
        });

    const openai = new OpenAI({
      apiKey: key
    });
    const chatCompletion = await openai.chat.completions.create({
      model: "gpt-3.5-turbo",
      messages: [{ role: "user", content: text }],
      functions: functions
    });
    return chatCompletion.choices[0].message.content;
  }



  /**
   * Generate text
   *
   * CONTINUE keword.
   *
   * result = CONTINUE text
   *
   */
  public static async continue(min: GBMinInstance, text: string, chatId) {
    let key;
    if (process.env.OPENAI_KEY) {
      key = process.env.OPENAI_KEY;
    }
    else {
      key = min.core.getParam(min.instance, 'Open AI Key', null);
    }

    if (!key) {
      throw new Error('Open AI Key not configured in .gbot.');
    }
    const openai = new OpenAI({
      apiKey: key
    });
    const chatCompletion = await openai.chat.completions.create({
      model: "gpt-3.5-turbo",
      messages: [{ role: "user", content: text }]
      
    });
    return chatCompletion.choices[0].message.content;
  }
}
