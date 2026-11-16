-- bld.bunker@modern: Bunker (mesh.bunker_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.bunker_modern','Bunker');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.bunker_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('berm','plinth','mat.turf',0,'','{"w":1.85,"d":1.85,"h":0.2,"steps":2,"inset":0.1,"tint":"#c8c8a8"}'),
  ('body','cylinder','mat.concrete',0,'','{"w":1.5,"h":0.62,"segments":6}'),
  ('slit_1','box','mat.crude',0,'','{"x":0.5694,"y":0.38,"z":0.3288,"yaw":60,"w":0.4,"d":0.03,"h":0.08,"far":0}'),
  ('slit_2','box','mat.crude',0,'','{"x":0,"y":0.38,"z":0.6575,"yaw":0,"w":0.4,"d":0.03,"h":0.08,"far":0}'),
  ('slit_3','box','mat.crude',0,'','{"x":-0.5694,"y":0.38,"z":0.3288,"yaw":-60,"w":0.4,"d":0.03,"h":0.08,"far":0}'),
  ('slit_4','box','mat.crude',0,'','{"x":-0.5694,"y":0.38,"z":-0.3288,"yaw":-120,"w":0.4,"d":0.03,"h":0.08,"far":0}'),
  ('slit_5','box','mat.crude',0,'','{"x":0.5694,"y":0.38,"z":-0.3288,"yaw":-240,"w":0.4,"d":0.03,"h":0.08,"far":0}'),
  ('roof','cylinder','mat.concrete',0,'','{"y":0.62,"w":1.74,"h":0.16,"segments":6}'),
  ('roof_band','cylinder','mat.cloth',1,'','{"y":0.64,"w":1.77,"h":0.11,"segments":6}'),
  ('roof_panel','cylinder','mat.cloth',1,'','{"y":0.78,"w":1,"h":0.03,"segments":6}'),
  ('cupola','dome','mat.cast_iron',0,'','{"x":0.15,"y":0.81,"z":0.05,"w":0.5,"h":0.26,"segments":8}'),
  ('mg','cylinder','mat.cast_iron',0,'','{"x":0.15,"y":0.91,"z":0.2,"w":0.05,"h":0.38,"segments":4,"pitch":90}'),
  ('periscope','cylinder','mat.steel',0,'','{"x":-0.35,"y":0.78,"z":-0.2,"w":0.06,"h":0.3,"segments":4,"far":0}'),
  ('entrance','box','mat.concrete',0,'','{"z":-0.82,"yaw":180,"w":0.5,"d":0.3,"h":0.48,"door":0.26,"door_h":0.4,"door_colour":"#3a3e44"}'),
  ('flag','banner','mat.cloth',1,'','{"x":-0.45,"y":0.78,"z":0.1,"h":1.1,"fw":0.45,"fh":0.3}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.bunker_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.bunker'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.bunker_modern'), (SELECT id FROM def WHERE key='mat.concrete'), 100);
