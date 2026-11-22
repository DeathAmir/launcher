import { useEffect, useMemo } from "react";
import { useQuery } from "../../../../hooks/query";
import { useGenericTempState } from "../../../../states/genericStates";
import { useServers } from "../../../../states/servers";
import { sortAndSearchInServerList } from "../../../../utils/helpers";
import { Server } from "../../../../utils/types";
import List from "../List";
import ServerItem from "./../Item";

const Partners = () => {
  const { startQuery, stopQuery } = useQuery();
  const { selected, servers, setSelected } = useServers();
  const { searchData } = useGenericTempState();

  useEffect(() => {
    return () => {
      stopQuery();
      setSelected(undefined);
    };
  }, []);

  const list = useMemo(() => {
    return sortAndSearchInServerList(servers.filter((server) => server.port === 7777 && (server.ip === "play.irautox.ir" || server.ip === "5.57.37.122")), searchData, true);
  }, [servers, searchData]);

  const onSelect = (server: Server) => {
    stopQuery();
    setSelected(server);
    startQuery(server, "internet");
  };

  return (
    <List
      data={list}
      renderItem={(item, index) => (
        <ServerItem
          isSelected={
            selected
              ? selected.ip === item.ip && selected.port === item.port
              : false
          }
          server={item}
          index={index}
          onSelect={(server) => onSelect(server)}
        />
      )}
    />
  );
};

export default Partners;
