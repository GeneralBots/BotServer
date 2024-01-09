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
| "General Bots" is a registered trademark of pragmatismo.com.br.                 |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

import Swagger from 'swagger-client';
import { google } from 'googleapis';
import { promisify } from 'util';
import { PubSub } from '@google-cloud/pubsub';
import Fs from 'fs';
import { GBLog, GBMinInstance, GBService } from 'botlib';
import { GBServer } from '../../../src/app.js';
import { SecService } from '../../security.gbapp/services/SecService.js';

/**
 * Support for Google Chat.
 */
export class GoogleChatDirectLine extends GBService {
  public static conversationIds = {};
  public pollInterval = 5000;
  public directLineClientName = 'DirectLineClient';

  public directLineClient: any;
  public GoogleChatSubscriptionName: string;
  public botId: string;
  public min: GBMinInstance;
  private directLineSecret: string;
  pubSubClient: any;
  GoogleChatApiKey: any;
  GoogleClientEmail: any;
  GoogleClientPrivateKey: any;
  GoogleProjectId: any;

  constructor (
    min: GBMinInstance,
    botId,
    directLineSecret,
    GoogleChatSubscriptionName,
    GoogleChatApiKey,
    GoogleClientEmail,
    GoogleClientPrivateKey,
    GoogleProjectId
  ) {
    super();

    this.min = min;
    this.botId = botId;
    this.directLineSecret = directLineSecret;
    this.GoogleChatSubscriptionName = GoogleChatSubscriptionName;
    this.GoogleChatApiKey = GoogleChatApiKey;
    this.GoogleClientEmail = GoogleClientEmail;
    this.GoogleClientPrivateKey = GoogleClientPrivateKey;
    this.GoogleProjectId = GoogleProjectId;

    this.pubSubClient = new PubSub({
      projectId: this.GoogleProjectId,
      credentials: { client_email: GoogleClientEmail, private_key: GoogleClientPrivateKey }
    });
  }

  public static async asyncForEach (array, callback) {
    for (let index = 0; index < array.length; index++) {
      await callback(array[index], index, array);
    }
  }

  public async setup (setUrl) {
    this.directLineClient = new Swagger({
      spec: JSON.parse(Fs.readFileSync('directline-3.0.json', 'utf8')),
      usePromise: true
    });
    const client = await this.directLineClient;

    client.clientAuthorizations.add(
      'AuthorizationBotConnector',
      new Swagger.ApiKeyAuthorization('Authorization', `Bearer ${this.directLineSecret}`, 'header')
    );

    if (setUrl) {
      try {
        const subscription = this.pubSubClient.subscription(this.GoogleChatSubscriptionName);
        subscription.on('message', this.receiver.bind(this));
      } catch (error) {
        GBLog.error(`Error initializing 3rd party GoogleChat provider(1) ${error.message}`);
      }
    }
  }

  public async resetConversationId (key) {
    GoogleChatDirectLine.conversationIds[key] = undefined;
  }

  public async check () {
    GBLog.info(`GBGoogleChat: Checking server...`);
  }

  public async receiver (message) {
    const event = JSON.parse(Buffer.from(message.data, 'binary').toString());

    let from = '';
    let fromName = '';
    let text;
    const threadName = event.message.thread.name;

    if (event['type'] === 'ADDED_TO_SPACE' && event['space']['singleUserBotDm']) {
    } else if (event['type'] === 'MESSAGE') {
      text = event.message.text;
      fromName = event.message.sender.displayName;
      from = event.message.sender.email;
      GBLog.info(`Received message from ${from} (${fromName}): ${text}.`);
    }
    message.ack();

    const sec = new SecService();
    const user = await sec.ensureUser(this.min.instance.instanceId, from, from, '', 'googlechat', fromName, from);

    await sec.updateConversationReferenceById(user.userId, threadName);

    GBLog.info(`GBGoogleChat: RCV ${from}: ${text})`);

    const client = await this.directLineClient;
    const conversationId = GoogleChatDirectLine.conversationIds[from];

    if (GoogleChatDirectLine.conversationIds[from] === undefined) {
      GBLog.info(`GBGoogleChat: Starting new conversation on Bot.`);
      const response = await client.Conversations.Conversations_StartConversation();
      const generatedConversationId = response.obj.conversationId;

      GoogleChatDirectLine.conversationIds[from] = generatedConversationId;

      this.pollMessages(client, generatedConversationId, threadName, from, fromName);
      this.inputMessage(client, generatedConversationId, threadName, text, from, fromName);
    } else {
      this.inputMessage(client, conversationId, threadName, text, from, fromName);
    }
  }

  public inputMessage (client, conversationId, threadName, text, from, fromName) {
    return client.Conversations.Conversations_PostActivity({
      conversationId: conversationId,
      activity: {
        textFormat: 'plain',
        text: text,
        type: 'message',
        mobile: from,
        from: {
          id: from,
          name: fromName
        },
        replyToId: from
      }
    });
  }

  public pollMessages (client, conversationId, threadName, from, fromName) {
    GBLog.info(`GBGoogleChat: Starting message polling(${from}, ${conversationId}).`);

    let watermark: any;

    const worker = async () => {
      try {
        const response = await client.Conversations.Conversations_GetActivities({
          conversationId: conversationId,
          watermark: watermark
        });
        watermark = response.obj.watermark;
        await this.printMessages(response.obj.activities, conversationId, threadName, from, fromName);
      } catch (err) {
        GBLog.error(`Error calling printMessages on GoogleChat channel ${err.data === undefined ? err : err.data}`);
      }
    };
    setInterval(worker, this.pollInterval);
  }

  public async printMessages (activities, conversationId, threadName, from, fromName) {
    if (activities && activities.length) {
      // Ignore own messages.

      activities = activities.filter(m => m.from.id === this.botId && m.type === 'message');

      if (activities.length) {
        // Print other messages.

        await GoogleChatDirectLine.asyncForEach(activities, async activity => {
          await this.printMessage(activity, conversationId, threadName, from, fromName);
        });
      }
    }
  }

  public async printMessage (activity, conversationId, threadName, from, fromName) {
    let output = '';

    if (activity.text) {
      GBLog.info(`GBGoogleChat: SND ${from}(${fromName}): ${activity.text}`);
      output = activity.text;
    }

    if (activity.attachments) {
      activity.attachments.forEach(attachment => {
        switch (attachment.contentType) {
          case 'image/png':
            GBLog.info(`Opening the requested image ${attachment.contentUrl}`);
            output += `\n${attachment.contentUrl}`;
            break;
          default:
            GBLog.info(`Unknown content type: ${attachment.contentType}`);
        }
      });
    }

    await this.sendToDevice(from, conversationId, threadName, output);
  }

  public async sendToDevice (from: string, conversationId: string, threadName, msg: string) {
    try {
      let threadParts = threadName.split('/');
      let spaces = threadParts[1];
      let threadKey = threadParts[3];
      const scopes = ['https://www.googleapis.com/auth/chat.bot'];

      const jwtClient = new google.auth.JWT(this.GoogleClientEmail, null, this.GoogleClientPrivateKey, scopes, null);
      await jwtClient.authorize();
      const chat = google.chat({ version: 'v1', auth: jwtClient });

      const res = await chat.spaces.messages.create({
        parent: `spaces/${spaces}`,
        threadKey: threadKey,
        requestBody: {
          text: msg
        }
      });
      GBLog.info(res);
      GBLog.info(`Message [${msg}] sent to ${from}: `);
    } catch (error) {
      GBLog.error(`Error sending message to GoogleChat provider ${error.message}`);
    }
  }

  public async sendToDeviceEx (to, conversationId, threadName, text, locale) {
    const minBoot = GBServer.globals.minBoot as any;

    text = await minBoot.conversationalService.translate(minBoot, text, locale);
    await this.sendToDevice(to, conversationId, threadName, text);
  }
}
