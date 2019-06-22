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

import urlJoin = require('url-join');

const Swagger = require('swagger-client');
const rp = require('request-promise');
import { GBLog, GBService } from 'botlib';
import * as request from 'request-promise-native';
import { GBServer } from '../../../src/app';

/**
 * Support for Whatsapp.
 */
export class WhatsappDirectLine extends GBService {
  public pollInterval = 5000;
  public directLineClientName = 'DirectLineClient';

  public directLineClient: any;
  public whatsappServiceKey: string;
  public whatsappServiceNumber: string;
  public whatsappServiceUrl: string;
  public botId: string;
  public watermark: string;

  public conversationIds = {};

  constructor(
    botId,
    directLineSecret,
    whatsappServiceKey,
    whatsappServiceNumber,
    whatsappServiceUrl
  ) {
    super();

    this.botId = botId;
    this.whatsappServiceKey = whatsappServiceKey;
    this.whatsappServiceNumber = whatsappServiceNumber;
    this.whatsappServiceUrl = whatsappServiceUrl;
    const fs = require('fs');

    this.directLineClient =
      new Swagger({
        spec: JSON.parse(fs.readFileSync('directline-3.0.json', 'utf8')),
        usePromise: true
      });
    this.directLineClient
      .then(async client => {

        client.clientAuthorizations.add(
          'AuthorizationBotConnector',
          new Swagger.ApiKeyAuthorization('Authorization', `Bearer ${directLineSecret}`, 'header')
        );

        const options = {
          method: 'POST',
          url: urlJoin(this.whatsappServiceUrl, 'webhook'),
          qs: {
            token: this.whatsappServiceKey,
            webhookUrl: `${GBServer.globals.publicAddress}/instances/${this.botId}/whatsapp`,
            set: true
          },
          headers: {
            'cache-control': 'no-cache'
          }
        };

        try {
          const result = request.post(options);
          GBLog.info(result);
        } catch (error) {
          GBLog.error(`Error initializing 3rd party Whatsapp provider(1) ${error.message}`);
        }
      });
  }

  public static async asyncForEach(array, callback) {
    for (let index = 0; index < array.length; index++) {
      await callback(array[index], index, array);
    }
  }

  public received(req, res) {
    const text = req.body.messages[0].body;
    const from = req.body.messages[0].author.split('@')[0];
    const fromName = req.body.messages[0].senderName;

    if (req.body.messages[0].fromMe) {
      return; // Exit here.
    }

    GBLog.info(`GBWhatsapp: RCV ${from}(${fromName}): ${text})`);

    const conversationId = this.conversationIds[from];

    this.directLineClient.then(client => {
      if (this.conversationIds[from] === undefined) {
        GBLog.info(`GBWhatsapp: Starting new conversation on Bot.`);
        client.Conversations.Conversations_StartConversation()
          .then(response => {
            return response.obj.conversationId;
          }).catch(err => {
            GBLog.error(`Error calling Conversations_StartConversation on Whatsapp channel ${err.data}`);
          })

          .then(generatedConversationId => {
            this.conversationIds[from] = generatedConversationId;
            this.inputMessage(client, generatedConversationId, text, from, fromName);

            this.pollMessages(client, generatedConversationId, from, fromName);
          })
          .catch(err => {
            GBLog.error(`Error starting conversation ${err.data}`);
          });
      } else {
        this.inputMessage(client, conversationId, text, from, fromName);
      }
      res.end();
    }).catch(err => {
      GBLog.error(`Error initializing DirectLine for Whatsapp channel ${err.data}`);
    });

  }

  public inputMessage(client, conversationId, text, from, fromName) {
    client.Conversations.Conversations_PostActivity({
      conversationId: conversationId,
      activity: {
        textFormat: 'plain',
        text: text,
        type: 'message',
        from: {
          id: from,
          name: fromName
        },
        replyToId: from
      }
    }).catch(err => {
      GBLog.error(`GBWhatsapp: Error receiving message: ${err.data}.`);
    });
  }

  public pollMessages(client, conversationId, from, fromName) {
    GBLog.info(`GBWhatsapp: Starting polling message for conversationId:
        ${conversationId}.`);

    let task: NodeJS.Timer;
    const worker = () => {
      clearInterval(task);
      client.Conversations.Conversations_GetActivities({
        conversationId: conversationId,
        watermark: this.watermark
      })
        .then(response => {
          this.watermark = response.obj.watermark;

          return response.obj.activities;
        })
        .catch(err => {
          GBLog.error(`Error calling Conversations_GetActivities on Whatsapp channel ${err.data}`);
        })
        .then(async activities => {
          await this.printMessages(activities, conversationId, from, fromName);
        })
        .catch(err => {
          GBLog.error(`Error calling printMessages on Whatsapp channel ${err.data}`);
        });
      task = setInterval(worker, this.pollInterval);
    };

    task = setInterval(worker, this.pollInterval);
  }

  public async printMessages(activities, conversationId, from, fromName) {
    if (activities && activities.length) {
      // Ignore own messages.

      activities = activities.filter(m => m.from.id === this.botId && m.type === 'message');

      if (activities.length) {
        // Print other messages.

        await WhatsappDirectLine.asyncForEach(activities, async activity => {
          await this.printMessage(activity, conversationId, from, fromName);
        });
      }
    }
  }

  public async printMessage(activity, conversationId, from, fromName) {
    let output = '';

    if (activity.text) {
      GBLog.info(`GBWhatsapp: SND ${from}(${fromName}): ${activity.text}`);
      output = activity.text;
    }

    if (activity.attachments) {
      activity.attachments.forEach(attachment => {
        switch (attachment.contentType) {
          case 'application/vnd.microsoft.card.hero':
            output += `\n${this.renderHeroCard(attachment)}`;
            break;

          case 'image/png':
            GBLog.info(`Opening the requested image ${attachment.contentUrl}`);
            output += `\n${attachment.contentUrl}`;
            break;
          default:
            GBLog.info(`Unknown content type: ${attachment.contentType}`);
        }
      });
    }

    await this.sendToDevice(from, output);
  }

  public renderHeroCard(attachment) {
    return `${attachment.content.title} - ${attachment.content.text}`;
  }

  public async sendToDevice(to, msg) {
    const options = {
      method: 'POST',
      url: urlJoin(this.whatsappServiceUrl, 'message'),
      qs: {
        token: this.whatsappServiceKey,
        phone: to,
        body: msg
      },
      headers: {
        'cache-control': 'no-cache'
      }
    };

    try {
      // tslint:disable-next-line: await-promise
      const result = await request.post(options);
      GBLog.info(result);
    } catch (error) {
      GBLog.error(`Error sending message to Whatsapp provider ${error.message}`);
    }
  }
}
