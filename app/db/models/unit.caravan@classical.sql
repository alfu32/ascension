-- unit.caravan@classical: Caravan (mesh.caravan_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.caravan_classical','Caravan');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.caravan_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('mule','quadruped','mat.hide',0,'','{"z":0.55,"h":0.82,"l":0.82,"w":0.27,"pace":0.95,"stride":24,"tint":"#b8a898"}'),
  ('shafts','box','mat.timber',0,'','{"x":-0.17,"y":0.42,"z":-0.15,"w":0.04,"d":0.04,"h":0.95,"pitch":84,"repeat":2,"dx":0.34}'),
  ('bed','box','mat.sawn_timber',0,'','{"y":0.38,"z":-0.55,"w":0.74,"d":0.95,"h":0.08}'),
  ('sides','box','mat.sawn_timber',0,'','{"x":-0.35,"y":0.46,"z":-0.55,"w":0.04,"d":0.95,"h":0.18,"repeat":2,"dx":0.7,"far":0}'),
  ('wheels','wheel','mat.timber',0,'','{"x":-0.42,"z":-0.55,"w":0.62,"t":0.07,"spokes":3,"repeat":2,"dx":0.84}'),
  ('bales','box','mat.cloth',1,'','{"x":-0.15,"y":0.46,"z":-0.7,"w":0.32,"d":0.42,"h":0.3}'),
  ('baskets','box','mat.thatch',0,'','{"x":0.17,"y":0.46,"z":-0.36,"w":0.28,"d":0.32,"h":0.22}'),
  ('jars','cylinder','mat.mud_plaster',0,'','{"x":0.15,"y":0.46,"z":-0.82,"w":0.16,"h":0.3,"taper":0.3,"segments":6,"far":0}'),
  ('driver','figure','mat.cloth',1,'','{"x":0.48,"z":0.7,"build":1.05,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.caravan_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.caravan'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.caravan_classical'), (SELECT id FROM def WHERE key='mat.timber'), 100);
