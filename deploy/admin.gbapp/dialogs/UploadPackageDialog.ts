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

"use strict";

import { IGBDialog } from  "botlib";
import { Prompts, UniversalBot, Session, ListStyle } from "botbuilder";
import { GBMinInstance } from "botlib";
var fs = require("fs");
var request = require("request");
var mkdirp = require("mkdirp");
var builder = require("botbuilder");
const logger = require('../base/winston');

export class AskDialog extends IGBDialog {
  static setup(bot: UniversalBot, min: GBMinInstance) {
    bot.dialog("/attachFile", [
      function(session, args, next) {
        logger.debug("/attachFile/F1: Start");
        if (session.privateConversationData.JWToken === undefined) {
          logger.error("/attachFile/F1: Undefined JWToken");
          session.endConversation(
            "Unable to store your attachments. Sorry for the inconvenience, please try again."
          );
        } else {
          if (session.privateConversationData.userRequest.text.length === 0) {
            if (
              session.privateConversationData.userRequest.attachments.length ===
              1
            ) {
              var txt =
                "I received your attachment. Please let me know how should I handle it.";
            } else {
              var txt =
                "I received your attachments. Please let me know how should I handle them.";
            }
            var msg = new builder.Message(session)
              .textFormat("markdown")
              .text(txt);
            builder.Prompts.text(session, msg);
          } else {
            next();
          }
        }
      },

      function(session, args, next) {
        logger.debug("/attachFile/F2: Start");
        if (!(args.response === null) && !(args.response === undefined)) {
          session.privateConversationData.userRequest.text = args.response;
        }

        var mkdirName =
          "work"
          
        mkdirp(mkdirName, function(err) {
          if (err) {
            logger.error(
              "/attachFile/F2: unable to create folder. Error->  " + err
            );
            session.endConversation(
              "Unable to store your attachments. Sorry for the inconvenience, please try again."
            );
          } else {
            if (!mkdirName.endsWith("/")) {
              mkdirName = mkdirName + "/";
            }
            session.privateConversationData.attachmentsToWrite =
              session.privateConversationData.userRequest.attachments.length -
              1;
            writeFileRequest(session, mkdirName);
          }
        });
      }
    ]);

    function writeFileRequest(session, mkdirName) {
      var options = {
        url:
          session.privateConversationData.userRequest.attachments[
            session.privateConversationData.attachmentsToWrite
          ].contentUrl,
        method: "GET",
        headers: {
          "Content-type":
            session.privateConversationData.userRequest.attachments[
              session.privateConversationData.attachmentsToWrite
            ].contentType
        }
      };
      // if (
      //   session.message.address.channelId === "skype" ||
      //   session.message.address.channelId === "msteams"
      // ) {
      //   options.headers.Authorization =
      //     "Bearer " + session.privateConversationData.JWToken;
      // }

      request(options, function(err, response, body) {
        if (err) {
          logger.error(err);
        } else {
          logger.trace(response.statusCode);

          var fileName =
            session.privateConversationData.userRequest.attachments[
              session.privateConversationData.attachmentsToWrite
            ].name;
          if (fs.existsSync(mkdirName + fileName)) {
            var fileType = fileName.substr(fileName.lastIndexOf(".")); //e.g. '.pdf'
            var fileSubName = fileName.substr(
              0,
              fileName.length - fileType.length
            ); //'name' if original fileName is 'name.pdf'
            var j = 1;
            while (
              fs.existsSync(mkdirName + fileSubName + "(" + j + ")" + fileType)
            ) {
              j += 1;
            }
            fileName = fileSubName + "(" + j + ")" + fileType;
          }
          session.privateConversationData.userRequest.attachments[
            session.privateConversationData.attachmentsToWrite
          ] = {
            name: fileName,
            contentUrl: mkdirName,
            contentType:
              session.privateConversationData.userRequest.attachments[
                session.privateConversationData.attachmentsToWrite
              ].contentType
          };
          fs.writeFile(
            mkdirName + fileName,
            body,
            { encoding: "binary" },
            function(err) {
              //{encoding: 'binary' , flag: 'wx'}
              if (err) {
                logger.error(
                  "/attachFile/F2: unable to save file. Error->  " + err
                );
                session.endConversation(
                  "Unable to store your attachments. Sorry for the inconvenience, please try again."
                );
              } else {
                session.privateConversationData.attachmentsToWrite -= 1;
                if (session.privateConversationData.attachmentsToWrite < 0) {
                  session.beginDialog("/textRequest");
                } else {
                  writeFileRequest(session, mkdirName);
                }
              }
            }
          );
        }
      });
    }
  }
}
