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

/**
 * @fileoverview General Bots server core.
 */

'use strict';

import {
  AutoIncrement,
  BelongsTo,
  BelongsToMany,
  Column,
  CreatedAt,
  DataType,
  ForeignKey,
  HasMany,
  IsUUID,
  Length,
  Model,
  PrimaryKey,
  Sequelize,
  Table,
  UpdatedAt
} from 'sequelize-typescript';

import { GuaribasChannel, GuaribasInstance } from '../../core.gbapp/models/GBModel';
import { GuaribasSubject } from '../../kb.gbapp/models';
import { GuaribasUser } from '../../security.gbapp/models';

/**
 * A conversation that groups many messages.
 */
@Table
export class GuaribasConversation extends Model<GuaribasConversation> {

  @PrimaryKey
  @AutoIncrement
  @Column
  public conversationId: number;

  @ForeignKey(() => GuaribasInstance)
  @Column
  public instanceId: number;

  @ForeignKey(() => GuaribasSubject)
  @Column
  public startSubjectId: number;

  @BelongsTo(() => GuaribasSubject)
  public startSubject: GuaribasSubject;

  @ForeignKey(() => GuaribasChannel)
  @Column
  public channelId: string;

  @Column public rateDate: Date;

  @Column(DataType.FLOAT)
  @Column
  public rate: number;

  @Column(DataType.STRING(512))
  @Column
  public feedback: string;

  @Column
  @CreatedAt
  declare public createdAt: Date;

  @Column public text: string;

  @ForeignKey(() => GuaribasUser)
  @Column
  public startedByUserId: number;

  @BelongsTo(() => GuaribasUser)
  public startedBy: GuaribasUser;
}

/**
 * A single message in a conversation.
 */
@Table
export class GuaribasConversationMessage extends Model<GuaribasConversationMessage> {

  @PrimaryKey
  @AutoIncrement
  @Column
  public conversationMessageId: number;

  @ForeignKey(() => GuaribasSubject)
  @Column
  public subjectId: number;

  @Column(DataType.TEXT)
  public content: string;

  @Column
  @CreatedAt
  declare public createdAt: Date;

  @Column
  @UpdatedAt
  declare public updatedAt: Date;

  //tslint:disable-next-line:no-use-before-declare
  @ForeignKey(() => GuaribasConversation)
  @Column
  public conversationId: number;

  //tslint:disable-next-line:no-use-before-declare
  @BelongsTo(() => GuaribasConversation)
  public conversation: GuaribasConversation;

  @ForeignKey(() => GuaribasInstance)
  @Column
  public instanceId: number;

  @ForeignKey(() => GuaribasUser)
  @Column
  public userId: number;

  @BelongsTo(() => GuaribasUser)
  public user: GuaribasUser;
}
