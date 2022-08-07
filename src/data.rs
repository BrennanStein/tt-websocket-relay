/*
TroupesTailConnect

exports.handler = async (event) => {
    // TODO implement
    const response = {
        statusCode: 200,
        body: JSON.stringify('Hello from Lambda!'),
    };
    return response;
};
*/

/*
TroupesTailHost

const AWS = require("aws-sdk");

const dynamo = new AWS.DynamoDB.DocumentClient();

exports.handler = async (event) => {
    let body;
    let statusCode = 200;
    const headers = {
      "Content-Type": "application/json"
    };
    
    console.log(event);
    
    let gameId = getGameId();
    let pid = 0; // host is always 0
    
    let payload = JSON.parse(event.body.payload);
    payload.gameId = gameId;
    
    
    try {
        await dynamo.put({
            TableName: "TroupesTailDB",
            Item: {
              gameId: gameId,
              players: [
                {
                  connectionId: event.connectionId,
                  pId: 0
                }
              ]
            }
          }).promise();
          
        await dynamo.put({
            TableName: "TroupesTailConnectionDB",
            Item: {
              connectionId: event.connectionId,
              gameId: gameId
            }
        }).promise();
          
        body = {
          messageId: event.body.messageId,
          action: event.body.action,
          gameId: gameId,
          sender: pid,
          recipients: event.body.recipients,
          payload: JSON.stringify(payload)
        };
    } catch (e) {
        statusCode = 400;
        body = e.message;
    }
    
    return body;
};

const getGameId = () => {
  const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
  
  let result = '';
  const charactersLength = chars.length;
  for ( let i = 0; i < 4; i++ ) {
      result += chars.charAt(Math.floor(Math.random() * chars.length));
  }

  return result;
};
*/

/*
TroupesTailDisconnect

const AWS = require("aws-sdk");

const dynamo = new AWS.DynamoDB.DocumentClient();

exports.handler = async (event) => {
    
    console.log(event);
    
    // A player disconnected!
    const playerConnId = event.connectionId
    try {
        // What game were they connected to?
        let connectionData = await dynamo.get({
            TableName: 'TroupesTailConnectionDB',
            Key: {
              connectionId: playerConnId
            }
        }).promise();
        const gameId = connectionData.Item.gameId;
        
        // Let's get that game's game data
        let gameData = await dynamo.get({
            TableName: 'TroupesTailDB',
            Key: {
              gameId: gameId
            }
        }).promise();
        let players = gameData.Item.players;
        
        
        // Let's find and remove players's data
        const playerConnDataIndex = players.findIndex((item, i) => item.connectionId == playerConnId);
        let playerConnData = players[playerConnDataIndex]
        
        // Remove the player from the conn DB now that we have their data
        await dynamo.delete({
            TableName: 'TroupesTailConnectionDB',
            Key: {
                connectionId: playerConnId
            }
        }).promise();
        
        
        // Were they the host?
        if (playerConnData.pId == 0) {
            // If so, remove the game data from the db!
            await dynamo.delete({
                TableName: 'TroupesTailDB',
                Key: {
                    gameId: gameId
                }
            }).promise();
            
            // Tell all other players still in the game their host disconnected
            // TODO
        } else {
            // If not, just remove the player data from the dbs
            players.splice(playerConnDataIndex, 1)
        
            await dynamo.put({
                TableName: 'TroupesTailDB',
                Item: {
                    gameId: gameId,
                    players: players
                }
            }).promise();
        }
    } catch (e) {
        
    }
};
 */

 /*
 TroupesTailAddGuest
 const AWS = require('aws-sdk');
const ddb = new AWS.DynamoDB.DocumentClient();

exports.handler = async (event) => {
    // copied from deck bomber add guest
    
    let statusCode = 200;
    const headers = {
      "Content-Type": "application/json"
    };
    
    var body = event.body;
    var messageJSON = JSON.parse(body.payload);
    var gameId = body.gameId;
    var newPId = body.recipients[0];
    var guestConnectionId = messageJSON.arg;
    
    const client = new AWS.ApiGatewayManagementApi({
        apiVersion: "2018-11-29",
        endpoint: "https://sxc11091vc.execute-api.us-west-2.amazonaws.com/test",
    });
      
    // Check whether we are able to add this guest
    if (newPId > 0) {
        // Get the game data
        var gameData = await ddb.get({
            TableName: 'TroupesTailDB',
            Key: {
                gameId: gameId
            }
        }).promise();
        
        // Add to players
        let players = gameData.Item.players;
        players.push({ 
            "connectionID" : guestConnectionId,
            "pId" : newPId
        })
        
        // Put
        await  ddb.put({
            TableName: 'TroupesTailDB',
            Item: {
                gameId : gameId,
                players: players
            },
        }).promise();
        
        await ddb.put({
            TableName: 'TroupesTailConnectionDB',
            Item: {
                connectionId: guestConnectionId,
                gameId: gameId
            }
        }).promise();
        
        // change the message from a string message to a gameInfo message
        messageJSON = {
            "gameId": gameId,
            "pId": newPId
        }
        body.payload = JSON.stringify(messageJSON);
        
    } 
    
    // Send the requesting guest the status of their request
    await client.postToConnection({
        ConnectionId: guestConnectionId,
        Data: JSON.stringify(body)
    }).promise();
    
    
}
*/

/*
TroupesTailSend

// ES5 example
const AWS = require('aws-sdk');
const dynamo = new AWS.DynamoDB.DocumentClient();
// const { ApiGatewayManagementApi } = require("@aws-sdk/client-apigatewaymanagementapi");
// const client = new ApiGatewayManagementApi({endpoint: "https://sxc11091vc.execute-api.us-west-2.amazonaws.com/test"});


exports.handler = async (event) => {
  let statusCode = 200;
  const headers = {
    "Content-Type": "application/json"
  };

  console.log(event);
  
  const body = event.body;
  let gameData = await dynamo.get({
    TableName: 'TroupesTailDB',
    Key: {
      gameId: body.gameId
    }
  }).promise();
  
  console.log(gameData);

  // grab all the connection ids from the list of recipients from the body
  let players = gameData.Item.players;
  let connectionIds = [];
  for (let i = 0; i <  body.recipients.length; i++) {
    let pId = body.recipients[i];
    let player = players.find(function(item, i) {
      return item.pId == pId;
    });
    console.log('connection ID: ' + player.connectionId + ' for pID ' + pId)
    connectionIds[i] = player.connectionId;
  }
  

  
  // Send the message to each connectionId
  for (let i = 0; i < connectionIds.length; i++) {
    let connectionId = connectionIds[i];
    
    const client = new AWS.ApiGatewayManagementApi({
      apiVersion: "2018-11-29",
      endpoint: "https://sxc11091vc.execute-api.us-west-2.amazonaws.com/test",
    });
    
    await client.postToConnection({
        ConnectionId: connectionId,
        Data: JSON.stringify(body)
    }).promise();
  }
*/

/*
TroupesTailJoin

exports.handler = async (event) => {
    // Process the message from the prospective guest requesting to join
    let statusCode = 200;
    const headers = {
      "Content-Type": "application/json"
    };
    console.log(event);
    
    var body = event.body;
    const senderConnectionId = event.connectionId; 
    console.log(body.payload);
    var messageJSON = JSON.parse(body.payload);
    var requestedRoom = messageJSON.arg;
    
    const client = new AWS.ApiGatewayManagementApi({
        apiVersion: "2018-11-29",
        endpoint: "https://sxc11091vc.execute-api.us-west-2.amazonaws.com/test",
      });
    
    // Get the game data
    var gameData = await ddb.get({
        TableName: 'TroupesTailDB',
        Key: {
            gameId: requestedRoom
        }
    }).promise();
    
    console.log(gameData);
    
    let roomNotFound = (JSON.stringify(gameData) == '{}');
    
    // If requested GameID not found, return that
    if (roomNotFound) {
        messageJSON.arg += " not Found";
        body.payload = JSON.stringify(messageJSON);
        // Change the message ID from "REQ_JOIN_ROOM" to "ROOM_NOT_FOUND"
        body.messageId++;
        
        await client.postToConnection({
            ConnectionId: senderConnectionId,
            Data: JSON.stringify(body)
        }).promise();
    } else {
        // GameData for the given roomCode exists
        let players = gameData.Item.players;
        let host = players.find(function(item, i) {
            return item.pId == 0;
        });
        // Set the payload to the new connectionId
        messageJSON.arg = senderConnectionId;
        body.payload = JSON.stringify(messageJSON);
        
        await client.postToConnection({
            ConnectionId: host.connectionId,
            Data: JSON.stringify(body)
        }).promise();
    }


}
*/