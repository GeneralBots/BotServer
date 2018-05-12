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
| but WITHOUT ANY WARRANTY; without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the                |
| GNU Affero General Public License for more details.                         |
|                                                                             |
| "General Bots" is a registered trademark of Pragmatismo.io.                 |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

const Path = require("path");
const Fs = require("fs");
const _ = require("lodash");
const Parse = require("csv-parse");
const Async = require("async");
const UrlJoin = require("url-join");
const Walk = require("fs-walk");
const logger = require("../../../src/logger");
const Swagger = require('swagger-client');
const rp = require('request-promise');
import * as request from "request-promise-native";

import { GBServiceCallback, GBService, IGBInstance } from "botlib";

export class WhatsappDirectLine extends GBService {

    pollInterval = 1000;
    directLineClientName = 'DirectLineClient';
    directLineSpecUrl = 'https://docs.botframework.com/en-us/restapi/directline3/swagger.json';
    directLineClient: any;
    whatsappServiceKey: string;
    whatsappServiceNumber: string;
    botId: string;

    constructor(botId, directLineSecret, whatsappServiceKey, whatsappServiceNumber) {

        super();

        this.botId = botId;
        this.whatsappServiceKey = whatsappServiceKey;
        this.whatsappServiceNumber = whatsappServiceNumber;

        // TODO: Migrate to Swagger 3.
        this.directLineClient = rp(this.directLineSpecUrl)
            .then(function (spec) {
                return new Swagger({
                    spec: JSON.parse(spec.trim()),
                    usePromise: true
                });
            })
            .then(function (client) {
                client.clientAuthorizations.add('AuthorizationBotConnector',
                    new Swagger.ApiKeyAuthorization('Authorization', 'Bearer ' +
                        directLineSecret, 'header'));
                return client;
            })
            .catch(function (err) {
                logger.error('Error initializing DirectLine client', err);
            });

    }

    received(req, res) {

        logger.info(`GBWhatsapp: Hook called. Event: ${req.body.event}, 
            muid: ${req.body.muid}, contact: ${req.body.contact.name},
            (${req.body.contact.uid}, ${req.body.contact.type})`);

        let conversationId = null; // req.body.cuid;
        let text = req.body.message.body.text;
        let from = req.body.contact.uid;
        let fromName = req.body.contact.name;

        this.directLineClient.then((client) => {

            if (conversationId == null) {

                logger.info(`GBWhatsapp: Starting new conversation on Bot.`);
                client.Conversations.Conversations_StartConversation()
                    .then((response) => {
                        return response.obj.conversationId;
                    })
                    .then((conversationId) => {

                        this.inputMessage(client, conversationId, text,
                            from, fromName);

                        this.pollMessages(client, conversationId,from, fromName);
                    })
                    .catch((err) => {
                        console.error('Error starting conversation', err);
                    });

            } else {
                this.inputMessage(client, conversationId, text,
                    from, fromName);

                this.pollMessages(client, conversationId, from, fromName);
            }
            res.end();
        });
    }


    inputMessage(client, conversationId, text, from, fromName) {

        client.Conversations.Conversations_PostActivity(
            {
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
            }).catch(function (err) {
                logger.error(`GBWhatsapp: Error receiving message: ${err}.`);
            });

    }


    pollMessages(client, conversationId,from, fromName){

        logger.info(`GBWhatsapp: Starting polling message for conversationId: 
        ${conversationId}.`);

        var watermark = null;
        setInterval(() => {
            client.Conversations.Conversations_GetActivities({
                conversationId:
                    conversationId, watermark: watermark
            })
                .then((response) => {
                    watermark = response.obj.watermark;
                    return response.obj.activities;
                })
                .then((activities) => {
                    this.printMessages(activities, from, fromName);
                });
        }, this.pollInterval);
    }

    printMessages(activities,from, fromName) {

        if (activities && activities.length) {

            // Ignore own messages.

            activities = activities.filter((m) => { return m.from.id === this.botId });

            if (activities.length) {

                // Print other messages.

                activities.forEach(activity => {
                    this.printMessage(activity, from, fromName);
                });
            }
        }
    }

    printMessage(activity, from, fromName) {

        let output: string;

        if (activity.text) {
            logger.info(`GBWhatsapp: MSG: ${activity.text}`);
            output = activity.text;
        }

        if (activity.attachments) {
            activity.attachments.forEach(function (attachment) {
                switch (attachment.contentType) {
                    case "application/vnd.microsoft.card.hero":
                        output += this.renderHeroCard(attachment);
                        break;

                    case "image/png":
                        logger.info('Opening the requested image ' + attachment.contentUrl);
                        output += `\n${attachment.contentUrl}`;
                        break;
                }
            });
        }

        this.sendToDevice(from, fromName, output);
    }

    renderHeroCard(attachment) {
        let output: string;
        let width = 70;

        let contentLine = function (content) {
            return ' '.repeat((width - content.length) / 2) +
                content +
                ' '.repeat((width - content.length) / 2);
        }

        output += '/' + '*'.repeat(width + 1);
        output += '*' + contentLine(attachment.content.title) + '*';
        output += '*' + ' '.repeat(width) + '*';
        output += '*' + contentLine(attachment.content.text) + '*';
        output += '*'.repeat(width + 1) + '/';
    }

    async sendToDevice(to, toName, msg) {
        var options = {
            method: 'POST',
            url: 'https://www.waboxapp.com/api/send/chat',
            qs:
                {
                    token: this.whatsappServiceKey,
                    uid: this.whatsappServiceNumber,
                    to: to,
                    custom_uid: 'GBZAP' + (new Date()).toISOString,
                    text: msg
                },
            headers:
                {
                    'cache-control': 'no-cache'
                }
        };

        const result = await request.get(options);
    }
}