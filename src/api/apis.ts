import api from "../api/config";
import { UpdateInfo } from "../states/app";
import { Log } from "../utils/logger";
import { PING_TIMEOUT_VALUE } from "../utils/query";
import { Server } from "../utils/types";

interface ApiResponse<T> {
  success: boolean;
  data: T;
  error?: string;
}

export const getCachedList = async (): Promise<ApiResponse<Server[]>> => {
  const server: Server = {
    ip: "play.irautox.ir", port: 7777, hostname: "IRAutoX",
    playerCount: 0, maxPlayers: 100, gameMode: "IRAutoX", language: "Persian",
    hasPassword: false, version: "0.3.7", usingOmp: false, partner: true,
    ping: PING_TIMEOUT_VALUE, players: [], password: "", rules: {},
  };
  const { useServers, usePersistentServers } = await import("../states/servers");
  useServers.getState().setServers([server]);
  usePersistentServers.getState().setFavoritesList([server]);
  usePersistentServers.getState().addToRecentlyJoined(server);
  return { success: true, data: [server] };
};

export const getUpdateInfo = async (): Promise<
  ApiResponse<UpdateInfo | undefined>
> => {
  try {
    const response = await api.get<UpdateInfo>("/launcher", { timeout: 5000 });
    return { success: true, data: response.data };
  } catch (error) {
    Log.debug("Failed to fetch update info:", error);
    return {
      success: false,
      data: undefined,
      error: error instanceof Error ? error.message : "Unknown error",
    };
  }
};
