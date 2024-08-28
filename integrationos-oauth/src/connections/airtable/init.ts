import axios from "axios";

import { DataObject, OAuthResponse } from "../../lib/types";
import qs from "qs";

export const base64UrlEncode = async (input: string) => {
	const byteArray = new TextEncoder().encode(input);

	let base64String = btoa(String.fromCharCode(...byteArray));

	base64String = base64String.replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");

	while (base64String.length % 4 !== 0) {
		base64String += "=";
	}

	return base64String;
};

export const init = async ({ body }: DataObject): Promise<OAuthResponse> => {
		try {
			const requestBody = {
				grant_type: "authorization_code",
				code: body.metadata?.code,
				client_id: body.clientId,
				client_secret: body.clientSecret,
				redirect_uri: body.metadata?.redirectUri,
				code_verifier: "eNmtK0mRHQ.-93QxgrniB7rb.-TZ_Tjbonygt2aqk1.ltiwXQHmNFeFJPh19MZwXzFDfmFMpUZbAFVtSAtChNU08R4txdDBi7EY6ZHiBp6I8F1drUHZR",
			};

			const authorizationToken = await base64UrlEncode(`${body.clientId}:${body.clientSecret}`);

			const response = await axios({
				method: "POST",
				headers: {
					"Content-Type": "application/x-www-form-urlencoded",
					"Authorization": `Basic ${authorizationToken}`
				},
				data: qs.stringify(requestBody),
				url: "https://airtable.com/oauth2/v1/token",
			});

			const {
				data: {
					token_type: tokenType,
					scope,
					access_token: accessToken,
					expires_in: expiresIn,
					refresh_token: refreshToken,
					refresh_expires_in: refreshExpiresIn,
				}
			} = response;

			return {
				accessToken,
				refreshToken,
				expiresIn,
				tokenType,
				meta: {
					scope,
					refreshExpiresIn
				}
			};
		} catch
			(error) {
			throw new Error(`Error fetching access token for Airtable: ${error}`);
		}
	}
;