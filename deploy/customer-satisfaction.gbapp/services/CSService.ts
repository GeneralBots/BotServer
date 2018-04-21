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

const logger = require("../../../src/logger");
const Path = require("path");
const Fs = require("fs");
const FsExtra = require("fs-extra");
const _ = require("lodash");
const Parse = require("csv-parse");
const Async = require("async");
const UrlJoin = require("url-join");
const Walk = require("fs-walk");
const WaitUntil = require("wait-until");

import { GBServiceCallback } from "botlib";
import { UrlJoin } from 'url-join';
import { GBDeployer } from "../../core.gbapp/services/GBDeployer";
import { GuaribasQuestionAlternate } from '../models';
import { GuaribasConversation } from '../../analytics.gblib/models';

export class CSService {

  resolveQuestionAlternate(
    instanceId: number,
    questionTyped: string,
    cb: GBServiceCallback<GuaribasQuestionAlternate>
  ) {
    GuaribasQuestionAlternate.findOne({
      where: {
        instanceId: instanceId,
        questionTyped: questionTyped
      }
    }).then((value: GuaribasQuestionAlternate) => {
      cb(value, null);
    });
  }

  insertQuestionAlternate(
    instanceId: number,
    questionTyped: string,
    questionText: string,
    cb: GBServiceCallback<GuaribasQuestionAlternate>
  ) {
    GuaribasQuestionAlternate.create({
      questionTyped: questionTyped,
      questionText: questionText
    }).then(item => {
      if (cb) {
        cb(item, null);
      }
    });
  }

  updateConversationRate(
    conversation: GuaribasConversation,
    rate: number,
    cb: GBServiceCallback<GuaribasConversation>
  ) {
    conversation.rate = rate;
    conversation.save().then((value: GuaribasConversation) => {
      cb(conversation, null);
    });
  }

}
