// taken from https://github.com/X-com/RealmShark/blob/realmshark/src/main/java/packets/PacketType.java
// might not be completely accurate

pub mod C2S {
    pub const TELEPORT: u8 = 1;
    pub const CLAIM_LOGIN_REWARD_MSG: u8 = 3;
    pub const REQUESTTRADE: u8 = 5;
    pub const JOINGUILD: u8 = 7;
    pub const PLAYERTEXT: u8 = 9;
    pub const USEITEM: u8 = 13;
    pub const GUILDREMOVE: u8 = 15;
    pub const PETUPGRADEREQUEST: u8 = 16;
    pub const INVDROP: u8 = 19;
    pub const OTHERHIT: u8 = 20;
    pub const ACTIVE_PET_UPDATE_REQUEST: u8 = 24;
    pub const ENEMYHIT: u8 = 25;
    pub const EDITACCOUNTLIST: u8 = 27;
    pub const PLAYERSHOOT: u8 = 30;
    pub const PONG: u8 = 31;
    pub const PET_CHANGE_SKIN_MSG: u8 = 33;
    pub const ACCEPTTRADE: u8 = 36;
    pub const CHANGEGUILDRANK: u8 = 37;
    pub const SQUAREHIT: u8 = 40;
    pub const USEPORTAL: u8 = 47;
    pub const QUEST_ROOM_MSG: u8 = 48;
    pub const RESKIN: u8 = 51;
    pub const RESET_DAILY_QUESTS: u8 = 52;
    pub const PET_CHANGE_FORM_MSG: u8 = 53;
    pub const INVSWAP: u8 = 55;
    pub const CHANGETRADE: u8 = 56;
    pub const CREATE: u8 = 57;
    pub const QUEST_REDEEM: u8 = 58;
    pub const CREATEGUILD: u8 = 59;
    pub const SETCONDITION: u8 = 60;
    pub const LOAD: u8 = 61;
    pub const MOVE: u8 = 62;
    pub const GOTOACK: u8 = 65;
    pub const HELLO: u8 = 74;
    pub const UPDATEACK: u8 = 81;
    pub const BUY: u8 = 85;
    pub const AOEACK: u8 = 89;
    pub const PLAYERHIT: u8 = 90;
    pub const CANCELTRADE: u8 = 91;
    pub const KEY_INFO_REQUEST: u8 = 94;
    pub const CHOOSENAME: u8 = 97;
    pub const QUEST_FETCH_ASK: u8 = 98;
    pub const CHECKCREDITS: u8 = 102;
    pub const GROUNDDAMAGE: u8 = 103;
    pub const GUILDINVITE: u8 = 104;
    pub const ESCAPE: u8 = 105;
    pub const QUEUE_CANCEL: u8 = 113;
    pub const REDEEM_EXALTATION_REWARD: u8 = 115;
    pub const FORGE_REQUEST: u8 = 118;
    pub const SHOOT_ACK: u8 = 121;
    pub const CHANGE_ALLYSHOOT: u8 = 122;
    pub const GET_PLAYERS_LIST_MESSAGE: u8 = 123;
    pub const MODERATOR_ACTION_MESSAGE: u8 = 124;
    pub const CREEP_MOVE_MESSAGE: u8 = 126;
    pub const CUSTOM_MAP_DELETE: u8 = 129;
    pub const CUSTOM_MAP_LIST: u8 = 131;
    pub const CREEP_HIT: u8 = 133;
    pub const PLAYER_CALLOUT: u8 = 134;
    pub const BUY_REFINEMENT: u8 = 136;
    pub const DASH: u8 = 137;
    pub const DASH_ACK: u8 = 138;
    pub const BUY_CUSTOMISATION_SOCKET: u8 = 140;
    pub const FAVOUR_PET: u8 = 145;
    pub const SKIN_RECYCLE: u8 = 146;
    pub const CLAIM_BATTLE_PASS: u8 = 149;
    pub const BOOST_BP_MILESTONE: u8 = 151;
    pub const CONVERT_SEASONAL_CHARACTER: u8 = 154;
    pub const RETITLE: u8 = 155;
    pub const SET_GRAVE_STONE: u8 = 156;
    pub const SET_ABILITY: u8 = 157;
    pub const EMOTE: u8 = 159;
    pub const BUY_EMOTE: u8 = 160;
    pub const SET_TRACKED_SEASON: u8 = 162;
    pub const CLAIM_MISSION: u8 = 163;
    pub const SET_DISCOVERABLE: u8 = 167;
    pub const UNLOCK_ENCHANTMENT_SLOT: u8 = 173;
    pub const UNLOCK_ENCHANTMENT: u8 = 175;
    pub const APPLY_ENCHANTMENT: u8 = 177;
    pub const ACTIVATE_CRUCIBLE: u8 = 180;
    pub const CRUCIBLE_REQUEST: u8 = 182;
    pub const UPGRADE_ENCHANTER: u8 = 185;
    pub const UPGRADE_ENCHANTMENT: u8 = 187;
    pub const REROLL_ALL_ENCHANTMENTS: u8 = 189;
    pub const RESET_ENCHANTMENT_REROLL_COUNT: u8 = 191;
    pub const CREATE_PARTY_MESSAGE: u8 = 200;
    pub const PARTY_ACTION: u8 = 207;
    pub const PARTY_INVITE_RESPONSE: u8 = 209;
}

pub mod S2C {
    pub const FAILURE: u8 = 0;
    pub const DELETE_PET: u8 = 4;
    pub const QUEST_FETCH_RESPONSE: u8 = 6;
    pub const PING: u8 = 8;
    pub const NEWTICK: u8 = 10;
    pub const SHOWEFFECT: u8 = 11;
    pub const SERVERPLAYERSHOOT: u8 = 12;
    pub const TRADEACCEPTED: u8 = 14;
    pub const GOTO: u8 = 18;
    pub const NAMERESULT: u8 = 21;
    pub const BUYRESULT: u8 = 22;
    pub const HATCH_PET: u8 = 23;
    pub const GUILDRESULT: u8 = 26;
    pub const TRADECHANGED: u8 = 28;
    pub const TRADEDONE: u8 = 34;
    pub const ENEMYSHOOT: u8 = 35;
    pub const PLAYSOUND: u8 = 38;
    pub const VERIFY_EMAIL: u8 = 39;
    pub const NEW_ABILITY: u8 = 41;
    pub const UPDATE: u8 = 42;
    pub const TEXT: u8 = 44;
    pub const RECONNECT: u8 = 45;
    pub const DEATH: u8 = 46;
    pub const ALLYSHOOT: u8 = 49;
    pub const KEY_INFO_RESPONSE: u8 = 63;
    pub const AOE: u8 = 64;
    pub const GLOBAL_NOTIFICATION: u8 = 66;
    pub const NOTIFICATION: u8 = 67;
    pub const CLIENTSTAT: u8 = 69;
    pub const DAMAGE: u8 = 75;
    pub const ACTIVEPETUPDATE: u8 = 76;
    pub const INVITEDTOGUILD: u8 = 77;
    pub const PETYARDUPDATE: u8 = 78;
    pub const PASSWORD_PROMPT: u8 = 79;
    pub const QUESTOBJID: u8 = 82;
    pub const PIC: u8 = 83;
    pub const REALM_HERO_LEFT_MSG: u8 = 84;
    pub const TRADESTART: u8 = 86;
    pub const EVOLVE_PET: u8 = 87;
    pub const TRADEREQUESTED: u8 = 88;
    pub const MAPINFO: u8 = 92;
    pub const LOGIN_REWARD_MSG: u8 = 93;
    pub const INVRESULT: u8 = 95;
    pub const QUEST_REDEEM_RESPONSE: u8 = 96;
    pub const ACCOUNTLIST: u8 = 99;
    pub const CREATE_SUCCESS: u8 = 101;
    pub const FILE: u8 = 106;
    pub const RESKIN_UNLOCK: u8 = 107;
    pub const NEW_CHARACTER_INFORMATION: u8 = 108;
    pub const UNLOCK_INFORMATION: u8 = 109;
    pub const QUEUE_INFORMATION: u8 = 112;
    pub const EXALTATION_BONUS_CHANGED: u8 = 114;
    pub const VAULT_UPDATE: u8 = 117;
    pub const FORGE_RESULT: u8 = 119;
    pub const FORGE_UNLOCKED_BLUEPRINTS: u8 = 120;
    pub const STATS: u8 = 139;
    pub const UNKNOWN147: u8 = 147;
    pub const DAMAGE_BOOST: u8 = 148;
    pub const CLAIM_BP_MILESTONE_RESULT: u8 = 150;
    pub const UNKNOWN164: u8 = 164;
    pub const UNKNOWN165: u8 = 165;
    pub const STASIS: u8 = 166;
    pub const REALM_SCORE_UPDATE: u8 = 169;
    pub const CLAIM_REWARDS_INFO_PROMPT: u8 = 170;
    pub const CLAIM_CHEST_REWARD: u8 = 171;
    pub const CHEST_REWARD_RESULT: u8 = 172;
    pub const UNKNOWN181: u8 = 181;
    pub const CRUCIBLE_RESPONSE: u8 = 183;
    pub const UNKNOWN190: u8 = 190;
    pub const PARTY_ACTION_RESULT: u8 = 204;
    pub const INCOMING_PARTY_INVITE: u8 = 208;
    pub const INCOMING_PARTY_MEMBER_INFO: u8 = 210;
    pub const PARTY_MEMBER_ADDED: u8 = 212;
    pub const PARTY_LIST_MESSAGE: u8 = 214;
    pub const PARTY_JOIN_REQUEST: u8 = 215;
    pub const PARTY_REQUEST_RESPONSE: u8 = 217;
    pub const FOR_RECONNECT: u8 = 218;
    pub const LOADING_SCREEN: u8 = 222;
}
