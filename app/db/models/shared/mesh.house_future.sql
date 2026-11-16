-- mesh.house_future: Future housing pod (shared sub-assembly)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.house_future','Future housing pod');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.house_future';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('walls','box',NULL,0,'','{"w":1.5,"d":1,"h":1.15}'),
  ('glass','box','mat.smart_glass',0,'','{"y":0.5,"w":1.52,"d":1.02,"h":0.3,"far":0}'),
  ('glow','box','mat.light_strip',0,'','{"y":0.04,"w":1.53,"d":1.03,"h":0.03,"far":0}'),
  ('fascia','box','mat.white_composite',1,'','{"y":1.17,"w":1.56,"d":1.06,"h":0.12}'),
  ('roof','flat_roof','mat.white_composite',0,'','{"y":1.29,"w":1.5,"d":1,"t":0.04,"overhang":0,"far":0}'),
  ('pod','dome','mat.white_composite',0,'','{"x":-0.35,"y":1.33,"w":0.62,"h":0.3,"segments":10,"far":0}'),
  ('solar','box','mat.solar_panel',0,'','{"x":0.38,"y":1.36,"w":0.6,"d":0.75,"h":0.03,"pitch":16,"far":0}'),
  ('door','box','mat.graphite',0,'','{"x":0.45,"z":0.5,"w":0.26,"d":0.03,"h":0.48,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.house_future'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
