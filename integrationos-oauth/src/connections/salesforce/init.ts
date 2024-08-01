import axios from "axios";
import qs from "qs";
import { DataObject, OAuthResponse } from "../../lib/types";
import { differenceInSeconds } from "../../lib/helpers";

const getListAllId = async (accessToken: string, url: string) => {
  const response = await axios.get(url, {
    headers: {
      Authorization: `Bearer ${accessToken}`,
    },
  });
  const {
    data: { listviews },
  } = response;

  if (listviews?.length) {
    const filteredListviews = listviews.filter((lv: any) =>
      lv.label.startsWith("All")
    );
    if (filteredListviews.length) {
      return filteredListviews[0].id;
    }
  }

  return null;
};

export const init = async ({ body }: DataObject): Promise<OAuthResponse> => {
  try {
    const {
      clientId,
      clientSecret,
      metadata: {
        code,
        formData: { SALESFORCE_DOMAIN }, // Example: https://flow-fun-2719.my.salesforce.com
        redirectUri,
      },
    } = body;
    const baseUrl = `${SALESFORCE_DOMAIN}/services/oauth2`;

    const requestBody = {
      grant_type: "authorization_code",
      code,
      redirect_uri: redirectUri,
      client_id: body.clientId,
      client_secret: body.clientSecret,
    };
    const response = await axios({
      url: `${baseUrl}/token`,
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      data: qs.stringify(requestBody),
    });

    const {
      data: {
        access_token: accessToken,
        refresh_token: refreshToken,
        token_type: tokenType,
      },
    } = response;

    // Get expiry time through introspection
    const introspection = await axios({
      url: `${baseUrl}/introspect`,
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      data: qs.stringify({
        client_id: clientId,
        client_secret: clientSecret,
        token: accessToken,
        token_type_hint: "access_token",
      }),
    });
    const {
      data: { exp: expiresAt },
    } = introspection;
    // Converting expiresAt to date object and getting difference in seconds
    const expiresIn = differenceInSeconds(new Date(expiresAt * 1000));

    // Get all listview ids and save in meta for getMany
    const listViewBaseUrl = `${SALESFORCE_DOMAIN}/services/data/v61.0/sobjects`;
    const contactListView = `${listViewBaseUrl}/contact/listviews`;
    const opportunityListView = `${listViewBaseUrl}/opportunity/listviews`;
    const leadListView = `${listViewBaseUrl}/lead/listviews`;
    const accountListView = `${listViewBaseUrl}/lead/listviews`;

    const contactListId = await getListAllId(accessToken, contactListView);
    const opportunityListId = await getListAllId(
      accessToken,
      opportunityListView
    );
    const leadListId = await getListAllId(accessToken, leadListView);
    const accountListId = await getListAllId(accessToken, accountListView);

    return {
      accessToken,
      refreshToken,
      expiresIn,
      tokenType,
      meta: {
        contactListId,
        opportunityListId,
        leadListId,
        accountListId,
      },
    };
  } catch (error) {
    throw new Error(`Error fetching access token for Salesforce: ${error}`);
  }
};
