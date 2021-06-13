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

import urlJoin = require('url-join');

const Swagger = require('swagger-client');
const rp = require('request-promise');
const fs = require('fs');
import { GBLog, GBMinInstance, GBService, IGBPackage } from 'botlib';
import { CollectionUtil } from 'pragmatismo-io-framework';
import * as request from 'request-promise-native';
import { GBServer } from '../../../src/app';
import { GBConversationalService } from '../../core.gbapp/services/GBConversationalService';
import { SecService } from '../../security.gbapp/services/SecService';
import { Messages } from '../strings';
const { google } = require('googleapis')
const chat = google.chat('v1');
const { promisify } = require('util');
chat.spaces.messages.createAsync = promisify(chat.spaces.messages.create);
const { PubSub } = require('@google-cloud/pubsub');

// Creates a client; cache this for further use
const subscriptionName = 'projects/eastern-amp-316323/topics/generalbots';
const timeout = 60;

/**
 * Support for GoogleChat.
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

  constructor(
    min: GBMinInstance,
    botId,
    directLineSecret,
    GoogleChatSubscriptionName,
    GoogleChatApiKey,
  ) {
    super();

    this.min = min;
    this.botId = botId;
    this.directLineSecret = directLineSecret;
    this.GoogleChatSubscriptionName = GoogleChatSubscriptionName;
    this.pubSubClient = new PubSub();
    this.GoogleChatApiKey = GoogleChatApiKey;

  }

  public static async asyncForEach(array, callback) {
    for (let index = 0; index < array.length; index++) {
      await callback(array[index], index, array);
    }
  }

  public async setup(setUrl) {

    this.directLineClient =
      new Swagger({
        spec: JSON.parse(fs.readFileSync('directline-3.0.json', 'utf8')),
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

  public async resetConversationId(key) {
    GoogleChatDirectLine.conversationIds[key] = undefined;
  }

  public async check() {

    GBLog.info(`GBGoogleChat: Checking server...`);
  }

  public async receiver(message) {

    const event = JSON.parse(Buffer.from(message.data, 'binary').toString());
    
    let from = '';
    let fromName='';
    let text;
    const threadName = event.message.thread.name;
       
    if (event['type'] === 'ADDED_TO_SPACE' && event['space']['singleUserBotDm'])
    {
      
    }else if(event['type'] === 'MESSAGE')
    {
      text = event.message.text;
      fromName = event.message.sender.displayName;
      from = event.message.sender.email;
      GBLog.info(`Received message from ${from} (${fromName}): ${text}.`);
    }
    message.ack();

    GBLog.info(`GBGoogleChat: RCV ${from}: ${text})`);

    const client = await this.directLineClient;
    const conversationId = GoogleChatDirectLine.conversationIds[threadName];

    if (GoogleChatDirectLine.conversationIds[threadName] === undefined) {
      GBLog.info(`GBGoogleChat: Starting new conversation on Bot.`);
      const response = await client.Conversations.Conversations_StartConversation();
      const generatedConversationId = response.obj.conversationId;

      GoogleChatDirectLine.conversationIds[threadName] = generatedConversationId;

      this.pollMessages(client, generatedConversationId, from, fromName);
      this.inputMessage(client, generatedConversationId, text, from, fromName);
    } else {

      this.inputMessage(client, conversationId, text, from, fromName);
    }
  }

  public inputMessage(client, conversationId, text, from, fromName) {
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

  public pollMessages(client, conversationId, from, fromName) {
    GBLog.info(`GBGoogleChat: Starting message polling(${from}, ${conversationId}).`);

    let watermark: any;

    const worker = async () => {
      try {
        const response = await client.Conversations.Conversations_GetActivities({
          conversationId: conversationId,
          watermark: watermark
        });
        watermark = response.obj.watermark;
        await this.printMessages(response.obj.activities, conversationId, from, fromName);
      } catch (err) {
        GBLog.error(`Error calling printMessages on GoogleChat channel ${err.data === undefined ? err : err.data}`);
      }
    };
    setInterval(worker, this.pollInterval);
  }

  public async printMessages(activities, conversationId, from, fromName) {
    if (activities && activities.length) {
      // Ignore own messages.

      activities = activities.filter(m => m.from.id === this.botId && m.type === 'message');

      if (activities.length) {
        // Print other messages.

        await GoogleChatDirectLine.asyncForEach(activities, async activity => {
          await this.printMessage(activity, conversationId, from, fromName);
        });
      }
    }
  }

  public async printMessage(activity, conversationId, from, fromName) {
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

    await this.sendToDevice(from, output);
  }

  public async sendToDevice(threadName: string, msg: string) {

    try {
      
      let threadParts = threadName.split('/');
      let spaces = threadParts[1];
      let threadKey = threadParts[3];
      
      const res = await chat.spaces.messages.createAsync({
        auth: this.GoogleChatApiKey,
        parent: `spaces/${spaces}`,
        threadKey: threadKey,
        body: {
          text: msg
        }
      });
      GBLog.info(res);
      GBLog.info(`Message [${msg}] sent to ${threadName}: `);
    } catch (error) {
      GBLog.error(`Error sending message to GoogleChat provider ${error.message}`);
    }
  }


  public async sendToDeviceEx(to, text, locale) {
    const minBoot = GBServer.globals.minBoot as any;

    text = await minBoot.conversationalService.translate(
      minBoot,
      text,
      locale
    );
    await this.sendToDevice(to, text);
  }
}